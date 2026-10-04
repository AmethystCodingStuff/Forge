//! Project-scoped tool primitives. Subprocess tools remain unavailable until
//! exact one-shot authorization can be safely reviewed and bound to each call.

use serde_json::{Value, json};
use std::fs;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PermissionLevel {
    Safe,
    Balanced,
    Autonomous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Risk {
    Safe,
    Balanced,
    Dangerous,
}

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("permission denied for {operation}: requires {required:?} permission")]
    Permission {
        operation: String,
        required: PermissionLevel,
    },
    #[error("path must stay within the project root")]
    PathEscape,
    #[error("tool input is invalid: {0}")]
    Input(String),
    #[error("tool failed: {0}")]
    Execution(String),
    #[error(
        "E_COMMAND_AUTHORIZATION_REQUIRED: no child process was started; exact one-shot authorization is unavailable"
    )]
    AuthorizationRequired,
}

pub struct ToolExecutor {
    root: PathBuf,
    permission: PermissionLevel,
}

impl ToolExecutor {
    pub fn new(root: PathBuf, permission: PermissionLevel) -> Result<Self, ToolError> {
        Ok(Self {
            root: root
                .canonicalize()
                .map_err(|error| ToolError::Execution(error.to_string()))?,
            permission,
        })
    }

    pub fn definitions() -> Vec<forge_models::ToolDefinition> {
        vec![
            definition(
                "read_file",
                "Read a UTF-8 file inside the project.",
                json!({"path":{"type":"string"}}),
            ),
            definition(
                "write_file",
                "Replace a UTF-8 file inside the project.",
                json!({"path":{"type":"string"},"content":{"type":"string"}}),
            ),
            definition(
                "execute_command",
                "Unavailable: subprocess execution requires a fresh exact one-shot approval.",
                json!({"command":{"type":"string"}}),
            ),
            definition(
                "git_status",
                "Unavailable: Git subprocesses require a fresh exact one-shot approval.",
                json!({}),
            ),
            definition(
                "git_diff",
                "Unavailable: Git subprocesses require a fresh exact one-shot approval.",
                json!({}),
            ),
        ]
    }

    pub fn execute(&self, name: &str, arguments: &Value) -> Result<Value, ToolError> {
        match name {
            "read_file" => self.read_file(required_string(arguments, "path")?),
            "write_file" => self.write_file(
                required_string(arguments, "path")?,
                required_string(arguments, "content")?,
            ),
            "execute_command" => self.execute_command(required_string(arguments, "command")?),
            "git_status" => self.git(&["status", "--short"]),
            "git_diff" => self.git(&["diff", "--no-ext-diff"]),
            _ => Err(ToolError::Input(format!("unknown tool: {name}"))),
        }
    }

    fn checked_path(&self, input: &str) -> Result<PathBuf, ToolError> {
        let path = Path::new(input);
        if path.is_absolute()
            || path.components().any(|part| {
                matches!(
                    part,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(ToolError::PathEscape);
        }
        Ok(self.root.join(path))
    }

    fn read_file(&self, input: &str) -> Result<Value, ToolError> {
        self.allow(Risk::Safe, "read_file")?;
        let path = self.checked_path(input)?;
        Ok(
            json!({"path": input, "content": fs::read_to_string(path).map_err(|error| ToolError::Execution(error.to_string()))?}),
        )
    }

    fn write_file(&self, input: &str, content: &str) -> Result<Value, ToolError> {
        self.allow(Risk::Balanced, "write_file")?;
        let path = self.checked_path(input)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| ToolError::Execution(error.to_string()))?;
        }
        fs::write(path, content).map_err(|error| ToolError::Execution(error.to_string()))?;
        Ok(json!({"path": input, "bytes_written": content.len()}))
    }

    fn execute_command(&self, _command: &str) -> Result<Value, ToolError> {
        self.allow(Risk::Balanced, "execute_command")?;
        Err(ToolError::AuthorizationRequired)
    }

    fn git(&self, _args: &[&str]) -> Result<Value, ToolError> {
        self.allow(Risk::Safe, "git")?;
        Err(ToolError::AuthorizationRequired)
    }

    fn allow(&self, risk: Risk, operation: &str) -> Result<(), ToolError> {
        let required = match risk {
            Risk::Safe => PermissionLevel::Safe,
            Risk::Balanced => PermissionLevel::Balanced,
            Risk::Dangerous => PermissionLevel::Autonomous,
        };
        if self.permission < required {
            return Err(ToolError::Permission {
                operation: operation.to_owned(),
                required,
            });
        }
        Ok(())
    }
}

fn definition(name: &str, description: &str, properties: Value) -> forge_models::ToolDefinition {
    forge_models::ToolDefinition {
        name: name.to_owned(),
        description: description.to_owned(),
        parameters: json!({"type":"object","properties":properties,"additionalProperties":false}),
    }
}

fn required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, ToolError> {
    value[key]
        .as_str()
        .ok_or_else(|| ToolError::Input(format!("{key} must be a string")))
}

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tests;
