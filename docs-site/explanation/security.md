# Security & Sandboxing

RexOS is built around running LLM-driven tool calls with guardrails.

## Workspace sandbox

The filesystem tools:

- only allow **relative** paths inside the workspace
- reject parent traversal (`..`)
- reject symlink-based escapes

## Shell tool

The shell tool:

- runs inside the workspace directory
- uses a minimal environment
- enforces a timeout

On Windows, it runs via PowerShell; on Unix, via bash.

## Web fetch (SSRF protection)

`web_fetch` defaults to denying loopback/private IP ranges to reduce SSRF risk.

For local testing you can explicitly allow private targets with `allow_private=true`.

## Future: approvals

RexOS has the structure to add “approval hooks” for higher-risk actions (network writes, destructive commands, etc.). This is intentionally conservative by default.

