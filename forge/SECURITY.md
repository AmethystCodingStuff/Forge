# Security policy

Report vulnerabilities privately to maintainers. The initial project-root checks, permission levels, prefix checks, and command timeouts are defense in depth—not a complete sandbox. API keys are read from environment variables and never emitted to model context, events, output, or memory.
