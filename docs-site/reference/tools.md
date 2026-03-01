# Tools Reference

RexOS exposes a small set of tools to the agent runtime.

## `fs_read`

Read a UTF-8 text file **relative to the workspace root**.

- rejects absolute paths
- rejects `..` traversal
- rejects symlink escapes

## `fs_write`

Write a UTF-8 text file **relative to the workspace root** (creates parent directories).

Same sandboxing rules as `fs_read`.

## `shell`

Run a shell command inside the workspace:

- Unix: runs via `bash -c`
- Windows: runs via PowerShell

RexOS enforces a timeout and runs with a minimal environment.

## `web_fetch`

Fetch an HTTP(S) URL and return a small response body.

By default it rejects loopback/private IPs (basic SSRF protection). For local testing you can set `allow_private=true`.

## `browser_*` (Playwright)

Browser tools enable headless browser automation via a Python Playwright bridge:

- `browser_navigate` / `browser_click` / `browser_type` / `browser_read_page` / `browser_screenshot` / `browser_close`

Notes:

- `browser_navigate` is SSRF-protected by default (denies loopback/private targets unless `allow_private=true`).
- `browser_screenshot` writes a PNG to a workspace-relative path (no absolute paths, no `..`, no symlink escapes).
- Requires Python + Playwright:

  ```bash
  python3 -m pip install playwright
  python3 -m playwright install chromium
  ```
