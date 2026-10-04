use super::*;
use pretty_assertions::assert_eq;

#[test]
fn rejects_paths_that_escape_project() {
    let temp = tempfile::tempdir().unwrap();
    let tools = ToolExecutor::new(temp.path().to_owned(), PermissionLevel::Balanced).unwrap();
    assert!(matches!(
        tools.execute("read_file", &json!({"path":"../secret"})),
        Err(ToolError::PathEscape)
    ));
}

#[test]
fn balanced_permission_allows_project_edits() {
    let temp = tempfile::tempdir().unwrap();
    let tools = ToolExecutor::new(temp.path().to_owned(), PermissionLevel::Balanced).unwrap();
    let result = tools
        .execute("write_file", &json!({"path":"notes.txt","content":"Forge"}))
        .unwrap();
    assert_eq!(result, json!({"path":"notes.txt","bytes_written":5}));
    assert_eq!(
        std::fs::read_to_string(temp.path().join("notes.txt")).unwrap(),
        "Forge"
    );
}

#[test]
fn subprocess_tools_fail_closed_without_launching_commands() {
    let temp = tempfile::tempdir().unwrap();
    let tools = ToolExecutor::new(temp.path().to_owned(), PermissionLevel::Balanced).unwrap();
    let sentinel = temp.path().join("must-not-exist");
    let command = format!("touch {}", sentinel.display());

    assert!(matches!(
        tools.execute("execute_command", &json!({"command":command})),
        Err(ToolError::AuthorizationRequired)
    ));
    assert!(!sentinel.exists());
    assert!(matches!(
        tools.execute("git_status", &json!({})),
        Err(ToolError::AuthorizationRequired)
    ));
    assert!(matches!(
        tools.execute("git_diff", &json!({})),
        Err(ToolError::AuthorizationRequired)
    ));
    assert!(!sentinel.exists());
}
