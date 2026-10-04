use std::process::{Command, Output};
use tempfile::tempdir;

fn forge(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(args)
        .output()
        .expect("built Forge binary should execute")
}

#[test]
fn update_is_unavailable_with_exact_error_and_exit_code() {
    let output = forge(&["update"]);

    assert_eq!(output.status.code(), Some(6));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).expect("stderr should be UTF-8"),
        "error[E_FEATURE_UNAVAILABLE]: `forge update` is unavailable.\n\
Update source, verification, opt-in, and rollback rules are not defined.\n\
No network request was made; no files were changed.\n"
    );
}

#[test]
fn doctor_is_unavailable_with_exact_error_and_exit_code() {
    let output = forge(&["doctor"]);

    assert_eq!(output.status.code(), Some(6));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).expect("stderr should be UTF-8"),
        "error[E_FEATURE_UNAVAILABLE]: `forge doctor` is unavailable.\n\
This command is not implemented yet.\n\
No files were changed.\n"
    );
}

#[test]
fn verify_with_mission_id_is_still_unavailable() {
    let output = forge(&["verify", "mission-42"]);

    assert_eq!(output.status.code(), Some(6));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).expect("stderr should be UTF-8"),
        "error[E_FEATURE_UNAVAILABLE]: `forge verify` is unavailable.\n\
This command is not implemented yet.\n\
No files were changed.\n"
    );
}

#[test]
fn malformed_command_is_invalid_usage_with_exit_code_two() {
    let output = forge(&["verify", "--extra"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).expect("stderr should be UTF-8"),
        "error[E_INVALID_USAGE]: invalid command or arguments: verify --extra\n"
    );
}

#[test]
fn help_and_version_remain_successful() {
    let help = forge(&["--help"]);
    assert_eq!(help.status.code(), Some(0));
    assert!(help.stderr.is_empty());
    let help = String::from_utf8(help.stdout).expect("help should be UTF-8");
    assert!(help.starts_with("Forge\n\nUsage: forge"));
    assert!(help.contains("Available:\n"));
    assert!(help.contains("Unavailable:\n"));
    assert!(help.contains("Unavailable:\n<OBJECTIVE>"));
    assert!(help.contains("E_COMMAND_AUTHORIZATION_REQUIRED/3"));
    assert!(
        !help
            .lines()
            .take_while(|line| *line != "Unavailable:")
            .any(|line| { line.contains("<OBJECTIVE>") || line.contains("<MISSION>") })
    );
    assert!(help.contains("6   Command unavailable\n"));

    let version = forge(&["--version"]);
    assert_eq!(version.status.code(), Some(0));
    assert!(version.stderr.is_empty());
    assert_eq!(
        String::from_utf8(version.stdout).expect("version should be UTF-8"),
        format!("forge {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn bare_mission_prints_usage_without_running_a_feature() {
    let output = forge(&["mission"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout).expect("mission usage should be UTF-8"),
        "Usage: forge mission <list|status|resume|pause> [MISSION_ID]\n\
Mission subcommands are unavailable.\n"
    );
}

#[test]
fn objective_fails_closed_noninteractively_before_any_command_starts() {
    let temp = tempdir().unwrap();
    let sentinel = temp.path().join("must-not-exist");
    let objective = format!("run touch {}", sentinel.display());
    let output = Command::new(env!("CARGO_BIN_EXE_forge"))
        .arg(objective)
        .current_dir(temp.path())
        .output()
        .expect("Forge binary should execute");

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).expect("stderr should be UTF-8"),
        "error[E_COMMAND_AUTHORIZATION_REQUIRED]: a fresh exact one-shot approval is required for objective.\n\
This build has no safe interactive exact-invocation approval flow.\n\
No provider request, subprocess, or file modification was started.\n"
    );
    assert!(!sentinel.exists());
}
