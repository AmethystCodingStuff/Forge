use forge_cli::{CliError, Command, MissionAction, parse_args};
use std::env;

fn main() {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let result = parse_args(&arguments).and_then(dispatch);
    if let Err(error) = result {
        std::process::exit(report_error(error));
    }
}

fn dispatch(command: Command) -> Result<(), CliError> {
    match command {
        Command::Help => {
            print!("{}", forge_cli::help_text());
            Ok(())
        }
        Command::Version => {
            println!("forge {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::Mission(None) => {
            print!("{}", forge_cli::mission_help_text());
            Ok(())
        }
        Command::Doctor => unavailable("doctor", "This command is not implemented yet."),
        Command::Mission(Some(action)) => {
            let label = match action {
                MissionAction::List => "mission list",
                MissionAction::Status => "mission status",
                MissionAction::Resume => "mission resume",
                MissionAction::Pause => "mission pause",
            };
            unavailable(label, "This command is not implemented yet.")
        }
        Command::Verify => unavailable("verify", "This command is not implemented yet."),
        Command::Review => unavailable("review", "This command is not implemented yet."),
        Command::Memory => unavailable("memory", "This command is not implemented yet."),
        Command::Update => unavailable(
            "update",
            "Update source, verification, opt-in, and rollback rules are not defined.",
        ),
        Command::Objective { .. } => Err(CliError::CommandAuthorizationRequired {
            operation: "objective",
            reason: "This build has no safe interactive exact-invocation approval flow.",
        }),
    }
}

fn unavailable(command: &'static str, reason: &'static str) -> Result<(), CliError> {
    Err(CliError::FeatureUnavailable { command, reason })
}

/// Render CLI errors consistently and map them to Forge-level exit codes.
fn report_error(error: CliError) -> i32 {
    match error {
        CliError::InvalidUsage(message) => {
            eprintln!("error[E_INVALID_USAGE]: {message}");
            2
        }
        CliError::CommandAuthorizationRequired { operation, reason } => {
            eprintln!(
                "error[E_COMMAND_AUTHORIZATION_REQUIRED]: a fresh exact one-shot approval is required for {operation}."
            );
            eprintln!("{reason}");
            eprintln!("No provider request, subprocess, or file modification was started.");
            3
        }
        CliError::FeatureUnavailable { command, reason } => {
            eprintln!("error[E_FEATURE_UNAVAILABLE]: `forge {command}` is unavailable.");
            eprintln!("{reason}");
            if command == "update" {
                eprintln!("No network request was made; no files were changed.");
            } else {
                eprintln!("No files were changed.");
            }
            6
        }
    }
}
