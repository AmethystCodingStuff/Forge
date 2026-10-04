use super::*;
use forge_models::{ModelRequest, ModelResponse, ToolCall};
use forge_tools::PermissionLevel;
use pretty_assertions::assert_eq;
use serde_json::json;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

struct ScriptedProvider {
    calls: Arc<AtomicUsize>,
    responses: Mutex<Vec<ModelResponse>>,
}

impl ModelProvider for ScriptedProvider {
    fn complete(&self, _: &ModelRequest) -> Result<ModelResponse, ModelError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.responses.lock().unwrap().remove(0))
    }
}

struct FakeExecutor {
    calls: Arc<AtomicUsize>,
    child_starts: Arc<AtomicUsize>,
    sentinel: PathBuf,
}

impl AgentToolExecutor for FakeExecutor {
    fn execute(&self, name: &str, _: &serde_json::Value) -> Result<serde_json::Value, ToolError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        match name {
            "write_file" => {
                std::fs::write(&self.sentinel, "unexpected tool execution")
                    .map_err(|error| ToolError::Execution(error.to_string()))?;
            }
            "execute_command" => {
                self.child_starts.fetch_add(1, Ordering::SeqCst);
                let executable = std::env::current_exe()
                    .map_err(|error| ToolError::Execution(error.to_string()))?;
                let output = Command::new(executable)
                    .arg("--version")
                    .output()
                    .map_err(|error| ToolError::Execution(error.to_string()))?;
                if !output.status.success() {
                    return Err(ToolError::Execution(
                        "fake child process exited unsuccessfully".to_owned(),
                    ));
                }
            }
            _ => {}
        }
        Ok(json!({"success":true}))
    }
}

fn blocked_responses() -> Vec<ModelResponse> {
    vec![
        ModelResponse {
            content: None,
            tool_calls: vec![
                ToolCall {
                    id: "write-sentinel".to_owned(),
                    name: "write_file".to_owned(),
                    arguments: json!({"path":"sentinel.txt","content":"VERIFIED"}),
                },
                ToolCall {
                    id: "start-child".to_owned(),
                    name: "execute_command".to_owned(),
                    arguments: json!({"command":"must not run"}),
                },
            ],
        },
        ModelResponse {
            content: Some("VERIFIED; all checks passed".to_owned()),
            tool_calls: vec![],
        },
    ]
}

fn assert_blocked_without_model_prose(result: &AgentResult) {
    assert_eq!(result.status, AgentStatus::BlockedNotVerified);
    assert!(result.model_response.is_empty());
    assert!(result.events.is_empty());
    assert_eq!(
        result.render(),
        "Forge status: BLOCKED\nForge verification: NOT_VERIFIED"
    );
    assert!(!result.render().contains("VERIFIED; all checks passed"));
}

#[test]
fn run_stops_before_fake_provider_executor_child_or_file_activity() {
    let temp = tempfile::tempdir().unwrap();
    let sentinel = temp.path().join("sentinel.txt");
    let provider_calls = Arc::new(AtomicUsize::new(0));
    let executor_calls = Arc::new(AtomicUsize::new(0));
    let child_starts = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider {
        calls: Arc::clone(&provider_calls),
        responses: Mutex::new(blocked_responses()),
    };
    let executor = FakeExecutor {
        calls: Arc::clone(&executor_calls),
        child_starts: Arc::clone(&child_starts),
        sentinel: sentinel.clone(),
    };
    let agent = Agent::with_executor(provider, executor, "test-model".to_owned());

    let result = agent.run("Inspect the project.".to_owned()).unwrap();

    assert_eq!(provider_calls.load(Ordering::SeqCst), 0);
    assert_eq!(executor_calls.load(Ordering::SeqCst), 0);
    assert_eq!(child_starts.load(Ordering::SeqCst), 0);
    assert!(!sentinel.exists());
    assert_blocked_without_model_prose(&result);
}

#[test]
fn public_constructor_run_stays_blocked_with_real_tool_executor() {
    let temp = tempfile::tempdir().unwrap();
    let sentinel = temp.path().join("sentinel.txt");
    let provider_calls = Arc::new(AtomicUsize::new(0));
    let provider = ScriptedProvider {
        calls: Arc::clone(&provider_calls),
        responses: Mutex::new(blocked_responses()),
    };
    let tools = ToolExecutor::new(temp.path().to_owned(), PermissionLevel::Balanced).unwrap();
    let agent = Agent::new(provider, tools, "test-model".to_owned());

    let result = agent.run("Inspect the project.".to_owned()).unwrap();

    assert_eq!(provider_calls.load(Ordering::SeqCst), 0);
    assert!(!sentinel.exists());
    assert_blocked_without_model_prose(&result);
}
