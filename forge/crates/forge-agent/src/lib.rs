//! Bounded, event-producing agent runtime for the Forge foundation.

use forge_models::{ModelError, ModelMessage, ModelProvider, ModelRequest};
use forge_tools::{ToolError, ToolExecutor};
use serde_json::json;
use thiserror::Error;

const MAX_TURNS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentEvent {
    MissionStarted { objective: String },
    ToolStarted { name: String },
    ToolCompleted { name: String, success: bool },
    MissionCompleted { verified: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentResult {
    pub summary: String,
    pub verified: bool,
    pub events: Vec<AgentEvent>,
}

#[derive(Debug, Error)]
pub enum AgentError {
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error(transparent)]
    Tool(#[from] ToolError),
    #[error("agent reached its {MAX_TURNS}-turn safety limit without a final response")]
    TurnLimit,
}

/// Runs a bounded model/tool conversation and records observable execution events.
pub struct Agent<P> {
    provider: P,
    tools: ToolExecutor,
    model: String,
}

impl<P: ModelProvider> Agent<P> {
    pub fn new(provider: P, tools: ToolExecutor, model: String) -> Self {
        Self {
            provider,
            tools,
            model,
        }
    }

    pub fn run(&self, objective: String) -> Result<AgentResult, AgentError> {
        let mut events = vec![AgentEvent::MissionStarted {
            objective: objective.clone(),
        }];
        let mut messages = vec![
            ModelMessage { role: "system".to_owned(), content: "You are Forge, a careful software engineer. Inspect before editing, use tools only when needed, run relevant verification, and give a concise evidence-based final result. Never claim verification without tool output.".to_owned() },
            ModelMessage { role: "user".to_owned(), content: objective },
        ];
        for _ in 0..MAX_TURNS {
            let response = self.provider.complete(&ModelRequest {
                model: self.model.clone(),
                messages: messages.clone(),
                tools: ToolExecutor::definitions(),
            })?;
            if response.tool_calls.is_empty() {
                let summary = response
                    .content
                    .unwrap_or_else(|| "The provider returned no final result.".to_owned());
                let verified = messages.iter().any(|message| {
                    message.role == "tool" && message.content.contains("\"success\":true")
                });
                events.push(AgentEvent::MissionCompleted { verified });
                return Ok(AgentResult {
                    summary,
                    verified,
                    events,
                });
            }
            for call in response.tool_calls {
                events.push(AgentEvent::ToolStarted {
                    name: call.name.clone(),
                });
                let output = self.tools.execute(&call.name, &call.arguments);
                events.push(AgentEvent::ToolCompleted {
                    name: call.name.clone(),
                    success: output.is_ok(),
                });
                messages.push(ModelMessage {
                    role: "assistant".to_owned(),
                    content: format!("Tool call {}: {}", call.id, call.name),
                });
                messages.push(ModelMessage { role: "tool".to_owned(), content: json!({"tool_call_id": call.id, "result": output.as_ref().map_err(ToString::to_string)}).to_string() });
                output?;
            }
        }
        Err(AgentError::TurnLimit)
    }
}

#[cfg(test)]
#[path = "agent_tests.rs"]
mod tests;
