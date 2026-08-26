use forge_agent::{Agent, AgentEvent};
use forge_models::OpenRouterProvider;
use forge_tools::{PermissionLevel, ToolExecutor};
use std::env;

fn main() {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.is_empty()
        || arguments
            .iter()
            .any(|argument| argument == "--help" || argument == "-h")
    {
        print_help();
        return;
    }
    let options = match CliOptions::parse(&arguments) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("forge: {error}");
            std::process::exit(2);
        }
    };
    let objective = options.objective;
    let model = options.model;
    let provider = options.provider;
    if provider != "openrouter" {
        eprintln!(
            "forge: unsupported provider `{provider}`; this foundation currently implements OpenRouter"
        );
        std::process::exit(2);
    }
    let api_key = match env::var("OPENROUTER_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("forge: set OPENROUTER_API_KEY to use OpenRouter");
            std::process::exit(2);
        }
    };
    let root = env::current_dir().unwrap_or_else(|error| {
        eprintln!("forge: cannot determine current directory: {error}");
        std::process::exit(1);
    });
    let tools = ToolExecutor::new(root, PermissionLevel::Balanced).unwrap_or_else(|error| {
        eprintln!("forge: cannot initialize tools: {error}");
        std::process::exit(1);
    });
    println!("⚒ Forge\n\nMission:\n{objective}\n\n◉ Planning and execution");
    match Agent::new(
        OpenRouterProvider::new(api_key, options.base_url),
        tools,
        model,
    )
    .run(objective)
    {
        Ok(result) => {
            for event in &result.events {
                if let AgentEvent::ToolCompleted { name, success } = event {
                    println!("{} {name}", if *success { "✓" } else { "✗" });
                }
            }
            println!(
                "\n{}\n\n{}",
                if result.verified {
                    "VERIFIED"
                } else {
                    "NOT VERIFIED"
                },
                result.summary
            );
        }
        Err(error) => {
            eprintln!("\nFAILED\n{error}");
            std::process::exit(1);
        }
    }
}

struct CliOptions {
    objective: String,
    provider: String,
    model: String,
    base_url: Option<String>,
}

impl CliOptions {
    fn parse(arguments: &[String]) -> Result<Self, String> {
        let mut objective = Vec::new();
        let mut provider = "openrouter".to_owned();
        let mut model = None;
        let mut base_url = None;
        let mut index = 0;
        while let Some(argument) = arguments.get(index) {
            match argument.as_str() {
                "--provider" => provider = required_value(arguments, &mut index, "--provider")?,
                "--model" => model = Some(required_value(arguments, &mut index, "--model")?),
                "--base-url" => {
                    base_url = Some(required_value(arguments, &mut index, "--base-url")?)
                }
                option if option.starts_with("--") => {
                    return Err(format!("unknown option `{option}`"));
                }
                _ => objective.push(argument.clone()),
            }
            index += 1;
        }
        Ok(Self {
            objective: (!objective.is_empty())
                .then(|| objective.join(" "))
                .ok_or("a mission objective is required")?,
            provider,
            model: model
                .ok_or("--model is required; Forge does not select a vendor model implicitly")?,
            base_url,
        })
    }
}

fn required_value(arguments: &[String], index: &mut usize, option: &str) -> Result<String, String> {
    *index += 1;
    arguments
        .get(*index)
        .cloned()
        .ok_or_else(|| format!("{option} requires a value"))
}

fn print_help() {
    println!(
        "⚒ Forge — The AI Software Engineer\n\nUsage:\n  forge <MISSION> --model MODEL [--provider openrouter] [--base-url URL]\n\nEnvironment:\n  OPENROUTER_API_KEY  API key for OpenRouter (never stored by Forge)\n\nForge runs a bounded, tool-mediated engineering loop in the current project."
    );
}

#[cfg(test)]
mod tests {
    use super::CliOptions;
    use pretty_assertions::assert_eq;

    #[test]
    fn separates_option_values_from_the_mission() {
        assert_eq!(
            CliOptions::parse(&[
                "Build authentication".to_owned(),
                "--model".to_owned(),
                "forge/reasoner".to_owned()
            ])
            .unwrap()
            .objective,
            "Build authentication"
        );
    }
}
