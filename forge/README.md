# ⚒ Forge — The AI Software Engineer

**PLAN → BUILD → VERIFY → SHIP**

Forge is an independent, CLI-first Rust software engineering agent. This initial foundation includes a provider-neutral model interface, OpenRouter support, a bounded tool-mediated runtime, and project-contained file/command tools.

```bash
cd forge
cargo run -p forge-cli -- --help
OPENROUTER_API_KEY=... cargo run -p forge-cli -- "Inspect this project and run its tests" --model forge/reasoner --provider openrouter
```

Forge reads API keys only from `OPENROUTER_API_KEY`; it does not persist them. A successful command is required before a run is labelled `VERIFIED`. Missions, durable memory, web view, browser QA, and remote access are deferred until this loop is stable. See [the architecture](docs/ARCHITECTURE.md).

## Development

Run `cargo fmt --check` and `cargo test`. Read [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).
