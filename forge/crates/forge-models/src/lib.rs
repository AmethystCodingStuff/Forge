//! Provider-neutral model requests and the initial OpenRouter implementation.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelResponse {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelRequest {
    pub model: String,
    pub messages: Vec<ModelMessage>,
    pub tools: Vec<ToolDefinition>,
}

/// A model backend that produces constrained tool calls or a final answer.
pub trait ModelProvider: Send + Sync {
    fn complete(&self, request: &ModelRequest) -> Result<ModelResponse, ModelError>;
}

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("model request failed: {0}")]
    Request(String),
    #[error("model response was invalid: {0}")]
    InvalidResponse(String),
}

#[derive(Debug, Clone)]
pub struct OpenRouterProvider {
    api_key: String,
    base_url: String,
    timeout: Duration,
}

impl OpenRouterProvider {
    pub const DEFAULT_BASE_URL: &'static str = "https://openrouter.ai/api/v1";

    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            api_key,
            base_url: base_url.unwrap_or_else(|| Self::DEFAULT_BASE_URL.to_owned()),
            timeout: Duration::from_secs(60),
        }
    }

    fn request_body(request: &ModelRequest) -> Value {
        let messages = request
            .messages
            .iter()
            .map(|message| json!({"role": message.role, "content": message.content}))
            .collect::<Vec<_>>();
        let tools = request.tools.iter().map(|tool| {
            json!({"type": "function", "function": {"name": tool.name, "description": tool.description, "parameters": tool.parameters}})
        }).collect::<Vec<_>>();
        json!({"model": request.model, "messages": messages, "tools": tools})
    }
}

impl ModelProvider for OpenRouterProvider {
    fn complete(&self, request: &ModelRequest) -> Result<ModelResponse, ModelError> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let response: Value = ureq::post(&url)
            .header("Authorization", &format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .config()
            .timeout_global(Some(self.timeout))
            .build()
            .send_json(Self::request_body(request))
            .map_err(|error| ModelError::Request(error.to_string()))?
            .body_mut()
            .read_json()
            .map_err(|error| ModelError::InvalidResponse(error.to_string()))?;
        let message = response["choices"][0]["message"].clone();
        if message.is_null() {
            return Err(ModelError::InvalidResponse(
                "missing choices[0].message".to_owned(),
            ));
        }
        let tool_calls = message["tool_calls"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|call| {
                let arguments = call["function"]["arguments"].as_str().unwrap_or("{}");
                Ok(ToolCall {
                    id: call["id"].as_str().unwrap_or_default().to_owned(),
                    name: call["function"]["name"]
                        .as_str()
                        .unwrap_or_default()
                        .to_owned(),
                    arguments: serde_json::from_str(arguments)
                        .map_err(|error| ModelError::InvalidResponse(error.to_string()))?,
                })
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        Ok(ModelResponse {
            content: message["content"].as_str().map(str::to_owned),
            tool_calls,
        })
    }
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod tests;
