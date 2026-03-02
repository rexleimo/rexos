# Tools Reference

RexOS exposes a small core toolset, plus a compatibility tool surface (aliases + reserved names) so you can reuse prompts/manifests written for common agent tool conventions.

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

## Compatibility aliases

These tool names exist for compatibility and map to RexOS built-ins:

- `file_read` → `fs_read`
- `file_write` → `fs_write`
- `file_list` → directory listing (workspace-relative; `.` is allowed)
- `shell_exec` → `shell`
- `apply_patch` → apply `*** Begin Patch` / `*** End Patch` patches (add/update/delete)
- `web_search` → DuckDuckGo HTML search (best-effort; returns a short text list)
- `memory_store` / `memory_recall` → shared KV store persisted in `~/.rexos/rexos.db`

## `image_analyze`

Analyze an image file in the workspace and return basic metadata as JSON (`format`, `width`, `height`, `bytes`).

Supported formats: PNG, JPEG, GIF.

## `location_get`

Return environment metadata as JSON (`os`, `arch`, `tz`, `lang`).

RexOS does not perform IP-based geolocation.

## Runtime collaboration and scheduling tools

These tools are implemented by the agent runtime (not by the standalone `Toolset`) and persist state in `~/.rexos/rexos.db`:

- `agent_spawn` / `agent_list` / `agent_find` / `agent_send` / `agent_kill`
- `task_post` / `task_claim` / `task_complete` / `task_list`
- `event_publish`
- `schedule_create` / `schedule_list` / `schedule_delete`
- `knowledge_add_entity` / `knowledge_add_relation` / `knowledge_query`

## Reserved tools (stubs)

The following tool names are defined but currently return `tool not implemented yet: <name>`:

`media_describe`, `media_transcribe`, `image_generate`,
`cron_create`, `cron_list`, `cron_cancel`,
`channel_send`,
`hand_list`, `hand_activate`, `hand_status`, `hand_deactivate`,
`a2a_discover`, `a2a_send`,
`text_to_speech`, `speech_to_text`,
`docker_exec`,
`process_start`, `process_poll`, `process_write`, `process_kill`, `process_list`,
`canvas_present`.
