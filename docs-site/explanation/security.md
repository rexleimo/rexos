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

## Browser tools

RexOS can run a headless browser via **CDP** by default (no Python), and can also use a legacy Playwright bridge backend.

- `browser_navigate` / `browser_click` / `browser_type` / `browser_press_key` / `browser_wait_for` / `browser_read_page` / `browser_screenshot` / `browser_close`

Security notes:

- `browser_navigate` is SSRF-checked similar to `web_fetch` (denies loopback/private targets unless `allow_private=true`).
- `browser_read_page` and `browser_screenshot` also enforce the same SSRF protection (unless you enabled `allow_private`).
- Screenshots are written to a **workspace-relative** path (no absolute paths, no `..`, no symlink escapes).

## Future: approvals

RexOS has the structure to add “approval hooks” for higher-risk actions (network writes, destructive commands, etc.). This is intentionally conservative by default.
