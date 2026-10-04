# ⚒ Forge — The AI Software Engineer

**PLAN → BUILD → VERIFY → SHIP** is the long-term product goal; verification-backed completion is not available yet.

Forge is an early-stage Rust workspace with a provider-neutral model interface and an OpenRouter adapter. The CLI currently provides help/version and truthful unavailable or authorization-required responses. Objective execution is unavailable: the CLI refuses it before provider setup, and the public library agent entry point is hard-disabled before any provider or tool use because the required authorization and verification pipeline is not implemented.

`Agent::run` returns a typed `BLOCKED` / `NOT_VERIFIED` result with no model response, events, provider request, or tool execution. There is no opaque verification-evidence pipeline, required-check runner, review pipeline, or mission-completion path yet. Git and command subprocess tools remain unavailable; the separate project file-tool API is not an OS sandbox.

```bash
cd forge
cargo run -p forge-cli -- --help
```

The OpenRouter adapter accepts its API key from `OPENROUTER_API_KEY`, but `Agent::run` does not invoke it. Missions, durable memory, browser QA, and remote access are roadmap items. See [the architecture](docs/ARCHITECTURE.md).

## Development

Run `cargo fmt --all -- --check` and `cargo test --workspace --locked`. Read [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).
