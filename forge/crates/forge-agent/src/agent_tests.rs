use super::*;
use forge_models::{ModelResponse, ToolCall};
use forge_tools::PermissionLevel;
use pretty_assertions::assert_eq;
use std::sync::Mutex;

struct ScriptedProvider {
    responses: Mutex<Vec<ModelResponse>>,
}
impl ModelProvider for ScriptedProvider {
    fn complete(&self, _: &ModelRequest) -> Result<ModelResponse, ModelError> {
        Ok(self.responses.lock().unwrap().remove(0))
    }
}

#[test]
fn runs_tool_then_reports_an_evidence_based_result() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("input.txt"), "evidence").unwrap();
    let provider = ScriptedProvider {
        responses: Mutex::new(vec![
            ModelResponse {
                content: None,
                tool_calls: vec![ToolCall {
                    id: "call_1".to_owned(),
                    name: "read_file".to_owned(),
                    arguments: json!({"path":"input.txt"}),
                }],
            },
            ModelResponse {
                content: Some("Read input.txt successfully.".to_owned()),
                tool_calls: vec![],
            },
        ]),
    };
    let agent = Agent::new(
        provider,
        ToolExecutor::new(temp.path().to_owned(), PermissionLevel::Balanced).unwrap(),
        "test-model".to_owned(),
    );
    let result = agent.run("Inspect the input.".to_owned()).unwrap();
    assert_eq!(
        result,
        AgentResult {
            summary: "Read input.txt successfully.".to_owned(),
            verified: false,
            events: vec![
                AgentEvent::MissionStarted {
                    objective: "Inspect the input.".to_owned()
                },
                AgentEvent::ToolStarted {
                    name: "read_file".to_owned()
                },
                AgentEvent::ToolCompleted {
                    name: "read_file".to_owned(),
                    success: true
                },
                AgentEvent::MissionCompleted { verified: false },
            ]
        }
    );
}
