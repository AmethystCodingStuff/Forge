//! Hard-disabled public agent facade pending authorization and verification.

use forge_models::{ModelError, ModelProvider};
use forge_tools::{ToolError, ToolExecutor};
use serde_json::Value;
use thiserror::Error;

const MAX_TURNS: usize = 8;

/// The only outcome currently available without a verifier-issued evidence pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    BlockedNotVerified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentEvent {
    MissionStarted { objective: String },
    ToolStarted { name: String },
    ToolCompleted { name: String, success: bool },
    MissionFinished { status: AgentStatus },
}

/// A blocked agent outcome paired with Forge-owned, non-success status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentResult {
    /// Empty while [`Agent::run`] is hard-disabled; no model prose is returned.
    pub model_response: String,
    /// The runtime cannot report a verified or completed outcome yet.
    pub status: AgentStatus,
    pub events: Vec<AgentEvent>,
}

impl AgentResult {
    /// Render Forge's authoritative status, including untrusted text only when present.
    pub fn render(&self) -> String {
        let (mission_status, verification_status) = match self.status {
            AgentStatus::BlockedNotVerified => ("BLOCKED", "NOT_VERIFIED"),
        };
        let mut rendered =
            format!("Forge status: {mission_status}\nForge verification: {verification_status}");
        if !self.model_response.is_empty() {
            rendered.push_str("\nGenerated model response (unverified; not Forge status):\n");
            rendered.push_str(&self.model_response);
        }
        rendered
    }
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

/// Agent facade. Its run entry point remains disabled until authorization and
/// evidence-backed verification are implemented.
pub struct Agent<P> {
    _provider: P,
    _tools: Box<dyn AgentToolExecutor>,
    _model: String,
}

#[allow(dead_code)]
trait AgentToolExecutor: Send + Sync {
    fn execute(&self, name: &str, arguments: &Value) -> Result<Value, ToolError>;
}

impl AgentToolExecutor for ToolExecutor {
    fn execute(&self, name: &str, arguments: &Value) -> Result<Value, ToolError> {
        ToolExecutor::execute(self, name, arguments)
    }
}

impl<P: ModelProvider> Agent<P> {
    pub fn new(provider: P, tools: ToolExecutor, model: String) -> Self {
        Self::with_executor(provider, tools, model)
    }

    fn with_executor<E: AgentToolExecutor + 'static>(provider: P, tools: E, model: String) -> Self {
        Self {
            _provider: provider,
            _tools: Box::new(tools),
            _model: model,
        }
    }

    pub fn run(&self, objective: String) -> Result<AgentResult, AgentError> {
        let _ = objective;
        let status = AgentStatus::BlockedNotVerified;
        Ok(AgentResult {
            model_response: String::new(),
            status,
            events: Vec::new(),
        })
    }
}

#[cfg(test)]
#[path = "agent_tests.rs"]
mod tests;
