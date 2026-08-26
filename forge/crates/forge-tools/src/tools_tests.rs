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
