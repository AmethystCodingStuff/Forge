use super::*;
use pretty_assertions::assert_eq;

#[test]
fn openrouter_uses_openai_compatible_tool_schema() {
    let request = ModelRequest {
        model: "provider/model".to_owned(),
        messages: vec![ModelMessage {
            role: "user".to_owned(),
            content: "inspect".to_owned(),
        }],
        tools: vec![ToolDefinition {
            name: "read_file".to_owned(),
            description: "Read a file".to_owned(),
            parameters: json!({"type": "object"}),
        }],
    };
    assert_eq!(
        OpenRouterProvider::request_body(&request),
        json!({
            "model": "provider/model",
            "messages": [{"role": "user", "content": "inspect"}],
            "tools": [{"type": "function", "function": {"name": "read_file", "description": "Read a file", "parameters": {"type": "object"}}}]
        })
    );
}
