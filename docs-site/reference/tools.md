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

## `browser_*` (CDP)

Browser tools enable headless browser automation via **Chrome DevTools Protocol (CDP)** (no Python by default):

- `browser_navigate` / `browser_click` / `browser_type` / `browser_press_key` / `browser_wait_for` / `browser_read_page` / `browser_screenshot` / `browser_close`

Notes:

- `browser_navigate` is SSRF-protected by default (denies loopback/private targets unless `allow_private=true`).
- Headless by default. To show a GUI window, pass `headless=false` to `browser_navigate` (or set `REXOS_BROWSER_HEADLESS=0` as a default).
- `browser_screenshot` writes a PNG to a workspace-relative path (no absolute paths, no `..`, no symlink escapes).
- Default backend is CDP and requires a local Chromium-based browser (Chrome/Chromium/Edge). If RexOS can’t find it, set `REXOS_BROWSER_CHROME_PATH`.
- Optional remote CDP: set `REXOS_BROWSER_CDP_HTTP` (example: `http://127.0.0.1:9222`).
- Optional legacy backend (Playwright bridge): set `REXOS_BROWSER_BACKEND=playwright` and install Python + Playwright:

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

## `media_describe`

Describe a media file in the workspace and return best-effort metadata as JSON (`kind`, `bytes`, `ext`).

## `media_transcribe`

Transcribe media into text.

For now this tool only supports **text transcript files** in the workspace (`.txt`, `.md`, `.srt`, `.vtt`) and returns JSON (`text`).

## `image_generate`

Generate an image asset from a prompt.

For now this tool outputs **SVG** to a workspace-relative `path` (use a `.svg` filename).

## Runtime collaboration and scheduling tools

These tools are implemented by the agent runtime (not by the standalone `Toolset`) and persist state in `~/.rexos/rexos.db`:

- `agent_spawn` / `agent_list` / `agent_find` / `agent_send` / `agent_kill`
- `hand_list` / `hand_activate` / `hand_status` / `hand_deactivate`
- `task_post` / `task_claim` / `task_complete` / `task_list`
- `event_publish`
- `schedule_create` / `schedule_list` / `schedule_delete`
- `cron_create` / `cron_list` / `cron_cancel`
- `channel_send` (outbox enqueue; use `rexos channel drain` to deliver)
- `knowledge_add_entity` / `knowledge_add_relation` / `knowledge_query`

## `channel_send`

Enqueue an outbound message into the outbox. Delivery happens out-of-band via the dispatcher:

- run once: `rexos channel drain`
- long-running: `rexos channel worker`

Supported channels:

- `console`: prints the message on drain
- `webhook`: posts JSON to `REXOS_WEBHOOK_URL`

## `hand_*`

Hands are small, curated “agent templates” that spawn a specialized agent instance.

- `hand_list`: list built-in Hands and whether they are active.
- `hand_activate`: activates a Hand and returns `{instance_id, agent_id, ...}`.
- `hand_status`: returns the current active instance (if any) for a `hand_id`.
- `hand_deactivate`: deactivates a Hand instance by `instance_id` (kills its underlying agent).

After `hand_activate`, you can use `agent_send` to talk to the returned `agent_id`.

## `a2a_*`

A2A tools let RexOS talk to external A2A-compatible agents:

- `a2a_discover`: fetches the agent card at `/.well-known/agent.json`
- `a2a_send`: sends a JSON-RPC `tasks/send` request to an A2A endpoint URL

Both are SSRF-protected by default; for local testing you can set `allow_private=true`.

## `speech_to_text`

Transcribe media into text.

MVP behavior: supports **text transcript files** (`.txt`, `.md`, `.srt`, `.vtt`) and returns JSON with `transcript` and `text`.

## `text_to_speech`

Convert text into an audio file.

MVP behavior: writes a short `.wav` file to the workspace (placeholder for real TTS).

## `docker_exec`

Run a command inside a one-shot Docker container with the workspace mounted.

- Disabled by default: set `REXOS_DOCKER_EXEC_ENABLED=1`
- Optional image override: `REXOS_DOCKER_EXEC_IMAGE` (default `alpine:3.20`)

## `process_*`

Start and interact with long-running processes:

- `process_start` / `process_poll` / `process_write` / `process_kill` / `process_list`

Processes run with the workspace as the working directory and a minimal environment.

## `canvas_present`

Save sanitized HTML to the workspace (under `output/`) and return metadata (`saved_to`, `canvas_id`, ...).

Scripts, event handlers (e.g. `onclick=`), and `javascript:` URLs are rejected.
