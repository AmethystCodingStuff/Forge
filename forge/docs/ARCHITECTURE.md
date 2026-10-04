# Forge architecture

## Current workspace

The Rust workspace separates provider models, a hard-disabled agent facade, project tools, core mission-domain rules, and CLI parsing. The crate boundaries exist, but the CLI does not currently dispatch objectives to the agent runtime.

`forge-models` defines provider-neutral requests and the OpenRouter adapter. `forge-agent` retains a public `Agent::run` facade, but that method is hard-disabled and returns before provider, tool, filesystem, or process use. `forge-tools` has basic relative-path file operations; its command and Git tools always return `E_COMMAND_AUTHORIZATION_REQUIRED` because there is no safe exact one-shot authorization flow. The CLI supports help/version, reports unfinished routes as unavailable, and refuses objective dispatch before provider setup or subprocess creation.

## Agent result and status

`Agent::run` returns only the typed `BlockedNotVerified` outcome: Forge status is `BLOCKED` and verification status is `NOT_VERIFIED`. Its model response is empty, it emits no execution events, and it makes no provider request or tool call. There is no verifier-issued evidence pipeline, required-check runner, or review pipeline, so objective execution remains unavailable.

The mission-domain completion gate also fails closed. Opaque evidence types and revision-binding interfaces exist, but there is no production workspace snapshotter, authorized check runner, evidence issuer, review-revision pipeline, or final resnapshot. Consequently, mission completion is unavailable; a caller-supplied `Verified` value or model-authored claim cannot complete a mission.

## Security boundaries and roadmap

Relative-path validation and permission levels are limited checks, not an OS sandbox or a race-safe filesystem capability. Subprocess execution and Git inspection are disabled; no command timeout, command-prefix execution policy, or child-process isolation is implemented. File operations do not confer confinement on a future child process.

Durable `.forge` missions and checkpoints, real verification and review, authenticated local APIs, browser QA, remote daemon pairing, and a safe interactive subprocess-approval flow remain roadmap items. Documentation and output must continue to distinguish these from implemented behavior.
