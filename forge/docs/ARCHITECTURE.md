# Forge architecture

## Foundation

Forge is a standalone Rust workspace with no dependency on another coding agent:

```text
forge-cli → forge-agent (bounded runtime + events)
                ├─ forge-models (provider abstraction + OpenRouter)
                └─ forge-tools (project-contained file, Git, and command tools)
```

A model response requests named tools with structured JSON or provides a final answer. The runtime stores system instruction, objective, and tool observations; it caps execution at eight turns. `MissionStarted`, `ToolStarted`, `ToolCompleted`, and `MissionCompleted` are presentation-independent events that future CLI, web, and mobile clients must consume from the same Forge process.

## Providers and safety

`ModelProvider` accepts neutral requests so role-based routing and additional providers can follow without changing the runtime. `OpenRouterProvider` uses the OpenAI-compatible `/chat/completions` API. `OPENROUTER_API_KEY` stays outside model messages and events.

Tools use `Safe`, `Balanced`, and `Autonomous` permissions. Paths must be relative to the project root; commands capture output, time out after 60 seconds, and block destructive/network-oriented prefixes unless autonomous permission is granted. This is defense in depth, not a complete sandbox.

## Verification and roadmap

Forge labels a run `VERIFIED` only after an executed command returns success. File work alone results in `NOT VERIFIED`; unrecoverable errors result in `FAILED`. Durable `.forge` missions and checkpoints, review/security guardians, authenticated local APIs, browser QA, and remote daemon pairing are deliberately deferred until the foundational loop is stable.
