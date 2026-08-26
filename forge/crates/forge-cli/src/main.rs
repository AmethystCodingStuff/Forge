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
    let objective = arguments
        .iter()
        .filter(|argument| !argument.starts_with("--"))
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    if objective.is_empty() {
        eprintln!("forge: a mission objective is required");
        std::process::exit(2);
    }
    let model =
        value_after(&arguments, "--model").unwrap_or_else(|| "openai/gpt-4.1-mini".to_owned());
    let provider = value_after(&arguments, "--provider").unwrap_or_else(|| "openrouter".to_owned());
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
        OpenRouterProvider::new(api_key, value_after(&arguments, "--base-url")),
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

fn value_after(arguments: &[String], flag: &str) -> Option<String> {
    arguments
        .iter()
        .position(|argument| argument == flag)
        .and_then(|index| arguments.get(index + 1))
        .cloned()
}
fn print_help() {
    println!(
        "⚒ Forge — The AI Software Engineer\n\nUsage:\n  forge <MISSION> [--provider openrouter] [--model MODEL] [--base-url URL]\n\nEnvironment:\n  OPENROUTER_API_KEY  API key for OpenRouter (never stored by Forge)\n\nForge runs a bounded, tool-mediated engineering loop in the current project."
    );
}
