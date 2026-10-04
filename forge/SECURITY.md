# Security policy

Forge is not an OS sandbox. Relative-path validation and permission-level checks are limited defenses; current file operations are not a race-safe or symlink-proof root capability. The `execute_command` and Git tools always fail with `E_COMMAND_AUTHORIZATION_REQUIRED`, so no child process starts through those tools. Public `Agent::run` is hard-disabled before provider or tool use and returns only `BLOCKED` / `NOT_VERIFIED` with no model response. This does not disable separately invoked provider or file-tool APIs. There is currently no command-prefix execution policy, subprocess timeout, or child isolation to rely on.

The OpenRouter provider keeps its API key in process memory and sends it in the provider authorization header; runtime prompts and agent events do not include the key. `OpenRouterProvider` derives `Debug`, so callers must not debug-format or log the provider object. Never commit credentials or store them in project files.

Report vulnerabilities privately to maintainers.
