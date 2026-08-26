//! Sandboxed-to-project tool primitives with explicit risk classification.

use serde_json::{Value, json};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
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
    #[error("tool timed out after {0:?}")]
    Timeout(Duration),
}

pub struct ToolExecutor {
    root: PathBuf,
    permission: PermissionLevel,
    command_timeout: Duration,
}

impl ToolExecutor {
    pub fn new(root: PathBuf, permission: PermissionLevel) -> Result<Self, ToolError> {
        Ok(Self {
            root: root
                .canonicalize()
                .map_err(|error| ToolError::Execution(error.to_string()))?,
            permission,
            command_timeout: Duration::from_secs(60),
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
                "Run a project command and return its captured output.",
                json!({"command":{"type":"string"}}),
            ),
            definition("git_status", "Show the project's Git status.", json!({})),
            definition("git_diff", "Show uncommitted Git changes.", json!({})),
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

    fn execute_command(&self, command: &str) -> Result<Value, ToolError> {
        self.allow(Risk::Balanced, "execute_command")?;
        if is_dangerous_command(command) {
            return Err(ToolError::Permission {
                operation: "dangerous command".to_owned(),
                required: PermissionLevel::Autonomous,
            });
        }
        let mut child = shell_command(command)
            .current_dir(&self.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| ToolError::Execution(error.to_string()))?;
        let start = Instant::now();
        while child
            .try_wait()
            .map_err(|error| ToolError::Execution(error.to_string()))?
            .is_none()
        {
            if start.elapsed() >= self.command_timeout {
                child
                    .kill()
                    .map_err(|error| ToolError::Execution(error.to_string()))?;
                return Err(ToolError::Timeout(self.command_timeout));
            }
            thread::sleep(Duration::from_millis(25));
        }
        let output = child
            .wait_with_output()
            .map_err(|error| ToolError::Execution(error.to_string()))?;
        Ok(
            json!({"success": output.status.success(), "exit_code": output.status.code(), "stdout": String::from_utf8_lossy(&output.stdout), "stderr": String::from_utf8_lossy(&output.stderr)}),
        )
    }

    fn git(&self, args: &[&str]) -> Result<Value, ToolError> {
        self.allow(Risk::Safe, "git")?;
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.root)
            .output()
            .map_err(|error| ToolError::Execution(error.to_string()))?;
        Ok(
            json!({"success": output.status.success(), "stdout": String::from_utf8_lossy(&output.stdout), "stderr": String::from_utf8_lossy(&output.stderr)}),
        )
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
fn is_dangerous_command(command: &str) -> bool {
    [
        "rm ",
        "del ",
        "git reset --hard",
        "git clean",
        "curl ",
        "wget ",
        "ssh ",
        "sudo ",
    ]
    .iter()
    .any(|prefix| command.trim_start().starts_with(prefix))
}
#[cfg(windows)]
fn shell_command(command: &str) -> Command {
    let mut shell = Command::new("cmd");
    shell.args(["/C", command]);
    shell
}
#[cfg(not(windows))]
fn shell_command(command: &str) -> Command {
    let mut shell = Command::new("sh");
    shell.args(["-lc", command]);
    shell
}

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tests;
