//! Command parsing and stable CLI error contracts for Forge.
//!
//! Objective syntax is parsed for compatibility, but dispatch fails closed until
//! an exact one-shot authorization flow is available. Other unfinished routes
//! fail truthfully as unavailable.

const HELP: &str = "Forge\n\n\
Usage: forge [--help] [--version] <command-or-objective>\n\n\
Available:\n\
  --help, -h\n\
  --version, -V\n\
Unavailable:\n\
  <OBJECTIVE> [--provider openrouter] [--model MODEL] [--base-url URL]\n\
    Blocked with E_COMMAND_AUTHORIZATION_REQUIRED/3; no provider request or subprocess starts.\n\
  doctor\n\
  mission [list|status|resume|pause] [MISSION_ID]\n\
  verify [MISSION_ID]\n\
  review [MISSION_ID]\n\
  memory\n\
  update\n\n\
Exit codes:\n\
  0   Command completed\n\
  1   Unexpected runtime failure\n\
  2   Invalid usage\n\
  3   Authorization or safety policy blocked\n\
  4   Configuration, environment, or persistence failure\n\
  5   Verification, review, or completion gate not met\n\
  6   Command unavailable\n\
  130 Interrupted\n";

const MISSION_HELP: &str = "Usage: forge mission <list|status|resume|pause> [MISSION_ID]\n\
Mission subcommands are unavailable.\n";

const DEFAULT_PROVIDER: &str = "openrouter";
const DEFAULT_MODEL: &str = "openai/gpt-4.1-mini";

#[derive(Debug, PartialEq, Eq)]
pub enum CliError {
    InvalidUsage(String),
    CommandAuthorizationRequired {
        operation: &'static str,
        reason: &'static str,
    },
    FeatureUnavailable {
        command: &'static str,
        reason: &'static str,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum MissionAction {
    List,
    Status,
    Resume,
    Pause,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Help,
    Version,
    Doctor,
    Mission(Option<MissionAction>),
    Verify,
    Review,
    Memory,
    Update,
    Objective {
        objective: String,
        provider: String,
        model: String,
        base_url: Option<String>,
    },
}

pub fn help_text() -> &'static str {
    HELP
}

pub fn mission_help_text() -> &'static str {
    MISSION_HELP
}

/// Parse supported top-level route shapes; objective dispatch remains fail-closed.
///
/// Known options are consumed separately so their values are not accidentally
/// appended to the objective. Unknown options and malformed feature routes are
/// typed invalid-usage errors; an arbitrary positional string is parsed as an
/// objective but is refused by the CLI dispatcher until authorization exists.
pub fn parse_args(args: &[String]) -> Result<Command, CliError> {
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return Ok(Command::Help);
    }
    if args == ["--version"] || args == ["-V"] {
        return Ok(Command::Version);
    }

    match args[0].as_str() {
        "--version" | "-V" => return Err(invalid_usage(args)),
        "doctor" => return exact_route(args, Command::Doctor),
        "mission" => return parse_mission(args),
        "verify" => return parse_optional_id_route(args, Command::Verify),
        "review" => return parse_optional_id_route(args, Command::Review),
        "memory" => return exact_route(args, Command::Memory),
        "update" => return exact_route(args, Command::Update),
        value
            if value.starts_with('-')
                && !matches!(value, "--provider" | "--model" | "--base-url") =>
        {
            return Err(invalid_usage(args));
        }
        _ => {}
    }

    parse_objective(args)
}

fn exact_route(args: &[String], command: Command) -> Result<Command, CliError> {
    if args.len() == 1 {
        Ok(command)
    } else {
        Err(invalid_usage(args))
    }
}

fn parse_optional_id_route(args: &[String], command: Command) -> Result<Command, CliError> {
    match args {
        [_] => Ok(command),
        [_, mission_id] if is_mission_id(mission_id) => Ok(command),
        _ => Err(invalid_usage(args)),
    }
}

fn parse_mission(args: &[String]) -> Result<Command, CliError> {
    let action = match args.get(1).map(String::as_str) {
        None if args.len() == 1 => None,
        Some("list") if args.len() == 2 => Some(MissionAction::List),
        Some("status") if has_optional_mission_id(args) => Some(MissionAction::Status),
        Some("resume") if has_optional_mission_id(args) => Some(MissionAction::Resume),
        Some("pause") if has_optional_mission_id(args) => Some(MissionAction::Pause),
        _ => return Err(invalid_usage(args)),
    };
    Ok(Command::Mission(action))
}

fn has_optional_mission_id(args: &[String]) -> bool {
    args.len() == 2 || (args.len() == 3 && is_mission_id(&args[2]))
}

fn is_mission_id(value: &str) -> bool {
    !value.is_empty() && !value.starts_with('-')
}

fn parse_objective(args: &[String]) -> Result<Command, CliError> {
    let mut objective_parts = Vec::new();
    let mut provider = DEFAULT_PROVIDER.to_owned();
    let mut model = DEFAULT_MODEL.to_owned();
    let mut base_url = None;
    let mut seen_provider = false;
    let mut seen_model = false;
    let mut seen_base_url = false;
    let mut index = 0;

    while index < args.len() {
        let argument = args[index].as_str();
        let (option, seen) = match argument {
            "--provider" => (Some("provider"), &mut seen_provider),
            "--model" => (Some("model"), &mut seen_model),
            "--base-url" => (Some("base-url"), &mut seen_base_url),
            value if value.starts_with('-') => return Err(invalid_usage(args)),
            _ => {
                objective_parts.push(argument);
                index += 1;
                continue;
            }
        };

        if *seen {
            return Err(invalid_usage(args));
        }
        *seen = true;
        let value = args
            .get(index + 1)
            .filter(|value| !value.starts_with('-'))
            .ok_or_else(|| invalid_usage(args))?
            .clone();
        match option.expect("recognized options always have a name") {
            "provider" => provider = value,
            "model" => model = value,
            "base-url" => base_url = Some(value),
            _ => unreachable!("only recognized options reach this match"),
        }
        index += 2;
    }

    if objective_parts.is_empty() {
        return Err(invalid_usage(args));
    }

    Ok(Command::Objective {
        objective: objective_parts.join(" "),
        provider,
        model,
        base_url,
    })
}

fn invalid_usage(args: &[String]) -> CliError {
    CliError::InvalidUsage(format!("invalid command or arguments: {}", args.join(" ")))
}

#[cfg(test)]
mod tests {
    use super::{CliError, Command, MissionAction, parse_args};

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn parses_help_and_version() {
        assert_eq!(parse_args(&args(&[])), Ok(Command::Help));
        assert_eq!(parse_args(&args(&["--help"])), Ok(Command::Help));
        assert_eq!(parse_args(&args(&["--version"])), Ok(Command::Version));
    }

    #[test]
    fn parses_feature_routes_and_optional_mission_ids() {
        assert_eq!(parse_args(&args(&["doctor"])), Ok(Command::Doctor));
        assert_eq!(parse_args(&args(&["memory"])), Ok(Command::Memory));
        assert_eq!(parse_args(&args(&["update"])), Ok(Command::Update));
        assert_eq!(parse_args(&args(&["mission"])), Ok(Command::Mission(None)));
        assert_eq!(
            parse_args(&args(&["mission", "list"])),
            Ok(Command::Mission(Some(MissionAction::List)))
        );
        assert_eq!(
            parse_args(&args(&["mission", "status", "mission-1"])),
            Ok(Command::Mission(Some(MissionAction::Status)))
        );
        assert_eq!(
            parse_args(&args(&["mission", "resume"])),
            Ok(Command::Mission(Some(MissionAction::Resume)))
        );
        assert_eq!(
            parse_args(&args(&["mission", "pause", "mission-1"])),
            Ok(Command::Mission(Some(MissionAction::Pause)))
        );
        assert_eq!(
            parse_args(&args(&["verify", "mission-1"])),
            Ok(Command::Verify)
        );
        assert_eq!(parse_args(&args(&["review"])), Ok(Command::Review));
    }

    #[test]
    fn preserves_upstream_objectives_and_consumes_option_values() {
        assert_eq!(
            parse_args(&args(&[
                "Inspect this project and run its tests",
                "--provider",
                "openrouter",
                "--model",
                "provider/model",
                "--base-url",
                "https://example.invalid/api/v1",
            ])),
            Ok(Command::Objective {
                objective: "Inspect this project and run its tests".to_owned(),
                provider: "openrouter".to_owned(),
                model: "provider/model".to_owned(),
                base_url: Some("https://example.invalid/api/v1".to_owned()),
            })
        );
        assert_eq!(
            parse_args(&args(&["--model", "provider/model", "Implement tests"])),
            Ok(Command::Objective {
                objective: "Implement tests".to_owned(),
                provider: "openrouter".to_owned(),
                model: "provider/model".to_owned(),
                base_url: None,
            })
        );
    }

    #[test]
    fn rejects_malformed_routes_and_unknown_options_as_invalid_usage() {
        for invalid in [
            args(&["verify", "--extra"]),
            args(&["mission", "delete"]),
            args(&["memory", "extra"]),
            args(&["--unknown"]),
            args(&["Implement tests", "--model"]),
        ] {
            assert!(matches!(
                parse_args(&invalid),
                Err(CliError::InvalidUsage(_))
            ));
        }
    }
}
