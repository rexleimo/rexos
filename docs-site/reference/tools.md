# Tools Reference

RexOS exposes a small core toolset, plus a compatibility tool surface (aliases + reserved names) so you can reuse prompts/manifests written for common agent tool conventions.

## Tool Index (60+)

The table of contents on this page lists **section headings**, not every tool name. Many tools are grouped under patterns like `browser_*`, `process_*`, `agent_*`, etc.

Use this index when writing prompts/manifests that need exact tool names:

### Core

`fs_read`, `fs_write`, `shell`, `web_fetch`, `pdf`, `pdf_extract`

### Browser

`browser_navigate`, `browser_back`, `browser_scroll`, `browser_click`, `browser_type`, `browser_press_key`, `browser_wait`, `browser_wait_for`, `browser_read_page`, `browser_run_js`, `browser_screenshot`, `browser_close`

### Compatibility aliases

`file_read`, `file_write`, `file_list`, `apply_patch`, `shell_exec`, `web_search`, `memory_store`, `memory_recall`

### Media

`image_analyze`, `image_generate`, `location_get`, `media_describe`, `media_transcribe`, `speech_to_text`, `text_to_speech`

### A2A

`a2a_discover`, `a2a_send`

### Sandbox & processes

`docker_exec`, `process_start`, `process_poll`, `process_write`, `process_kill`, `process_list`, `canvas_present`

### Runtime collaboration & scheduling

`agent_spawn`, `agent_list`, `agent_find`, `agent_send`, `agent_kill`, `hand_list`, `hand_activate`, `hand_status`, `hand_deactivate`, `task_post`, `task_claim`, `task_complete`, `task_list`, `event_publish`, `schedule_create`, `schedule_list`, `schedule_delete`, `cron_create`, `cron_list`, `cron_cancel`, `channel_send`, `workflow_run`, `knowledge_add_entity`, `knowledge_add_relation`, `knowledge_query`

## Running these examples

Most examples below are written as:

- a **tool call** JSON payload (the arguments object), and
- a **prompt** you can paste into `loopforge agent run`.

Quick runner:

=== "Bash (macOS/Linux)"
    ```bash
    loopforge agent run --workspace . --prompt "<PASTE PROMPT HERE>"
    ```

=== "PowerShell (Windows)"
    ```powershell
    loopforge agent run --workspace . --prompt "<PASTE PROMPT HERE>"
    ```

## `fs_read`

Read a UTF-8 text file **relative to the workspace root**.

- rejects absolute paths
- rejects `..` traversal
- rejects symlink escapes

### Example

Tool call:

```json
{ "path": "README.md" }
```

Prompt:

```text
Use fs_read to read README.md, then write notes/readme_summary.md with a 5-bullet summary.
```

## `fs_write`

Write a UTF-8 text file **relative to the workspace root** (creates parent directories).

Same sandboxing rules as `fs_read`.

### Example

Tool call:

```json
{ "path": "notes/hello.md", "content": "Hello from RexOS\\n" }
```

Prompt:

```text
Use fs_write to create notes/hello.md with a short hello message and today's date.
```

## `shell`

Run a shell command inside the workspace:

- Unix: runs via `bash -c`
- Windows: runs via PowerShell

RexOS enforces a timeout and runs with a minimal environment.

### Example

Tool call:

```json
{ "command": "echo READY && ls" }
```

Prompt:

```text
Use shell to run a safe command (echo READY and list the workspace). Write notes/shell_output.txt with the full output.
```

## `web_fetch`

Fetch an HTTP(S) URL and return a small response body.

By default it rejects loopback/private IPs (basic SSRF protection). For local testing you can set `allow_private=true`.

If `truncated=true`, RexOS returns a **head+tail** snippet with the marker `[...] middle omitted [...]` and includes both `bytes` (returned) and `total_bytes` (original).

### Example

Tool call:

```json
{ "url": "https://example.com", "timeout_ms": 20000, "max_bytes": 200000 }
```

Prompt:

```text
Use web_fetch to fetch https://example.com. Write notes/web_fetch_example.md with: status, content_type, and the first 200 characters of body.
```

## `pdf`

Extract text from a workspace PDF file (best-effort).

Arguments:

- `path` (required): workspace-relative `.pdf` path
- `pages` (optional): page selector (1-indexed), e.g. `"1"`, `"1-3"`, `"2,4-6"`
- `max_pages` (optional): default 10, max 50
- `max_chars` (optional): default 12000, max 50000

Returns JSON:

- `path`
- `text` (possibly truncated)
- `truncated` (bool)
- `bytes` (file size)
- `pages_total`
- `pages` (the selector string, or null)
- `pages_extracted`

### Example

Tool call:

```json
{ "path": "samples/dummy.pdf", "pages": "1-2", "max_pages": 10, "max_chars": 12000 }
```

Prompt:

```text
Use pdf (or pdf_extract) to extract text from samples/dummy.pdf (pages=1-2). Then write notes/pdf_excerpt.md with: (1) a 6-bullet summary, (2) key terms, (3) any garbled/missing parts you notice. Only use extracted text; do not invent.
```

See also: [PDF Summary case task](../examples/case-tasks/pdf-summarize.md).

## `browser_*` (CDP)

Browser tools enable headless browser automation via **Chrome DevTools Protocol (CDP)** (no Python by default):

- `browser_navigate` / `browser_back` / `browser_scroll` / `browser_click` / `browser_type` / `browser_press_key` / `browser_wait` / `browser_wait_for` / `browser_read_page` / `browser_run_js` / `browser_screenshot` / `browser_close`

Notes:

- `browser_navigate` is SSRF-protected by default (denies loopback/private targets unless `allow_private=true`).
- Headless by default. To show a GUI window, pass `headless=false` to `browser_navigate` (or set `REXOS_BROWSER_HEADLESS=0` as a default).
- `browser_screenshot` writes a PNG to a workspace-relative path (no absolute paths, no `..`, no symlink escapes).
- Default backend is CDP and requires a local Chromium-based browser (Chrome/Chromium/Edge). If RexOS can’t find it, set `REXOS_BROWSER_CHROME_PATH`.
- Optional remote CDP: set `REXOS_BROWSER_CDP_HTTP` (example: `http://127.0.0.1:9222`).
- Optional remote tab mode: set `REXOS_BROWSER_CDP_TAB_MODE=reuse` to skip `/json/new` and reuse an existing page target (default: `new`).
- Loopback CDP HTTP (`127.0.0.1` / `localhost`) bypasses proxy settings to avoid corporate proxy misconfig breaking local automation.
- Optional legacy backend (Playwright bridge): set `REXOS_BROWSER_BACKEND=playwright` and install Python + Playwright:

  ```bash
  python3 -m pip install playwright
  python3 -m playwright install chromium
  ```

`browser_wait` is a selector-only helper (compat). Prefer `browser_wait_for` when you need to wait for **selector or text**.

`browser_run_js` is useful for extracting structured values (like a specific heading) when selectors are tricky. Use it carefully on untrusted pages.

### `browser_navigate`

Starts (or reuses) a browser session and navigates to a URL.

Tool call:

```json
{ "url": "https://example.com", "timeout_ms": 30000, "headless": false }
```

Prompt:

```text
Use browser_navigate to open https://example.com (headless=false). Then save a screenshot to .rexos/browser/example.png and close the browser.
```

See also: [GUI Smoke Check](../how-to/browser-use-cases/gui-smoke-check.md), [Baidu Weather](../how-to/browser-use-cases/baidu-weather.md).

### `browser_back`

Go back in history (requires an active session).

Tool call:

```json
{}
```

Prompt:

```text
Use browser_navigate to open https://example.com, then open https://www.iana.org/domains/reserved, then call browser_back and confirm the URL is back to example.com.
```

### `browser_scroll`

Scroll the page (requires an active session).

Tool call:

```json
{ "direction": "down", "amount": 800 }
```

Prompt:

```text
Use browser_navigate to open https://example.com, then call browser_scroll down by 800, take a screenshot to .rexos/browser/scroll.png, then close the browser.
```

### `browser_click`

Click an element by CSS selector (best-effort fallback: match link/button text).

Tool call:

```json
{ "selector": "More information" }
```

Prompt:

```text
Use browser_navigate to open https://example.com, then browser_click \"More information\". Save a screenshot to .rexos/browser/click.png and close the browser.
```

### `browser_type`

Type into an input element (requires an active session).

Tool call:

```json
{ "selector": "input[name=\"wd\"]", "text": "北京 今天天气" }
```

Prompt:

```text
Use browser_navigate to open https://www.baidu.com. Wait for input[name=\"wd\"], then browser_type \"北京 今天天气\" into it, then press Enter. Save a screenshot and close the browser.
```

See also: [Baidu Weather](../how-to/browser-use-cases/baidu-weather.md).

### `browser_press_key`

Send a key press (optionally focus a selector first).

Tool call:

```json
{ "selector": "input[name=\"wd\"]", "key": "Enter" }
```

Prompt:

```text
On a search page, use browser_press_key with key=Enter to submit. If the site blocks automation, fall back to opening a direct results URL.
```

### `browser_wait`

Wait for a selector (compat helper).

Tool call:

```json
{ "selector": "#content_left", "timeout_ms": 30000 }
```

Prompt:

```text
Use browser_wait to wait for a results container selector, then read the page text.
```

### `browser_wait_for`

Wait for a selector **or** a text substring.

Tool call:

```json
{ "selector": "#content_left", "text": "天气", "timeout_ms": 30000 }
```

Prompt:

```text
Use browser_wait_for to wait until either #content_left exists or the page contains the text \"天气\". Then read_page and summarize.
```

### `browser_read_page`

Extract visible text and basic metadata (title/url).

Tool call:

```json
{}
```

Prompt:

```text
After navigating, use browser_read_page and write notes/page.txt with the first 2000 characters of content.
```

### `browser_run_js`

Run a JavaScript expression and return its value (use carefully).

Tool call:

```json
{ "expression": "document.title" }
```

Prompt:

```text
Use browser_run_js to return document.title, then write it to notes/title.txt.
```

### `browser_screenshot`

Save a PNG screenshot to the workspace (default: `.rexos/browser/screenshot.png`).

Tool call:

```json
{ "path": ".rexos/browser/page.png" }
```

Prompt:

```text
After navigating, use browser_screenshot to save evidence to .rexos/browser/page.png.
```

### `browser_close`

Close the browser session (safe to call multiple times).

Tool call:

```json
{}
```

Prompt:

```text
At the end of any browser workflow, call browser_close to clean up.
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

### `file_read`

Tool call:

```json
{ "path": "README.md" }
```

Prompt:

```text
Use file_read to read README.md, then write notes/readme_summary.md with 5 bullets.
```

### `file_write`

Tool call:

```json
{ "path": "notes/hello.txt", "content": "hello\\n" }
```

Prompt:

```text
Use file_write to create notes/hello.txt with a short message.
```

### `file_list`

Tool call:

```json
{ "path": "." }
```

Prompt:

```text
Use file_list to list files in the workspace root, then write notes/files.md with the listing.
```

### `shell_exec`

Tool call:

```json
{ "command": "echo hi", "timeout_seconds": 60 }
```

Prompt:

```text
Use shell_exec to run a safe command and write notes/shell_exec.txt with the output.
```

### `apply_patch`

Tool call:

```json
{
  "patch": "*** Begin Patch\\n*** Add File: notes/patched.txt\\n+hello from apply_patch\\n*** End Patch\\n"
}
```

Prompt:

```text
Use apply_patch to add notes/patched.txt with one line of text, then fs_read it back to confirm.
```

### `web_search`

Tool call:

```json
{ "query": "RexOS harness long-running agents", "max_results": 5 }
```

Prompt:

```text
Use web_search to find 5 results for \"RexOS harness long-running agents\". Write notes/search.md with titles + URLs.
```

### `memory_store`

Tool call:

```json
{ "key": "demo.favorite_color", "value": "blue" }
```

Prompt:

```text
Use memory_store to save key=demo.favorite_color value=blue. Then use memory_recall to fetch it and write notes/memory.md with the value.
```

### `memory_recall`

Tool call:

```json
{ "key": "demo.favorite_color" }
```

Prompt:

```text
Use memory_recall to read demo.favorite_color and print the value.
```

## `image_analyze`

Analyze an image file in the workspace and return basic metadata as JSON (`format`, `width`, `height`, `bytes`).

Supported formats: PNG, JPEG, GIF.

### Example

Tool call:

```json
{ "path": ".rexos/browser/page.png" }
```

Prompt:

```text
Use image_analyze on .rexos/browser/page.png and write notes/image_meta.json with the returned JSON.
```

## `location_get`

Return environment metadata as JSON (`os`, `arch`, `tz`, `lang`).

RexOS does not perform IP-based geolocation.

### Example

Tool call:

```json
{}
```

Prompt:

```text
Use location_get and write notes/env.json with the returned JSON.
```

## `media_describe`

Describe a media file in the workspace and return best-effort metadata as JSON (`kind`, `bytes`, `ext`).

### Example

Tool call:

```json
{ "path": "notes/readme_summary.md" }
```

Prompt:

```text
Use media_describe on notes/readme_summary.md and write notes/media_meta.json with the returned JSON.
```

## `media_transcribe`

Transcribe media into text.

For now this tool only supports **text transcript files** in the workspace (`.txt`, `.md`, `.srt`, `.vtt`) and returns JSON (`text`).

### Example

Tool call:

```json
{ "path": "samples/transcript.txt" }
```

Prompt:

```text
Use fs_write to create samples/transcript.txt with 3 short lines of dialogue. Then use media_transcribe to read it and write notes/transcript.md with the returned text.
```

## `image_generate`

Generate an image asset from a prompt.

For now this tool outputs **SVG** to a workspace-relative `path` (use a `.svg` filename).

### Example

Tool call:

```json
{ "prompt": "A simple SVG badge that says RexOS", "path": "assets/rexos_badge.svg" }
```

Prompt:

```text
Use image_generate to create assets/rexos_badge.svg. Then fs_read the file and write notes/badge_preview.md with the first 20 lines.
```

## Runtime collaboration and scheduling tools

These tools are implemented by the agent runtime (not by the standalone `Toolset`) and persist state in `~/.rexos/rexos.db`:

- `agent_spawn` / `agent_list` / `agent_find` / `agent_send` / `agent_kill`
- `hand_list` / `hand_activate` / `hand_status` / `hand_deactivate`
- `task_post` / `task_claim` / `task_complete` / `task_list`
- `event_publish`
- `schedule_create` / `schedule_list` / `schedule_delete`
- `cron_create` / `cron_list` / `cron_cancel`
- `channel_send` (outbox enqueue; use `loopforge channel drain` to deliver)
- `knowledge_add_entity` / `knowledge_add_relation` / `knowledge_query`

### `agent_spawn`

Create an agent session record (persisted) and return its details.

Tool call:

```json
{ "name": "Helper", "system_prompt": "You are a concise assistant." }
```

Prompt:

```text
Use agent_spawn to create an agent named Helper with a short system prompt. Then call agent_list and write notes/agents.json with the result.
```

### `agent_list`

Tool call:

```json
{}
```

Prompt:

```text
Use agent_list and write notes/agents.json with the JSON output.
```

### `agent_find`

Tool call:

```json
{ "query": "helper" }
```

Prompt:

```text
Use agent_find with query=helper and write notes/agent_find.json with the result.
```

### `agent_send`

Tool call:

```json
{ "agent_id": "<agent_id>", "message": "Summarize the workspace README in 3 bullets." }
```

Prompt:

```text
Use agent_spawn to create an agent, capture its agent_id, then use agent_send to ask it a question. Save the response to notes/agent_reply.md.
```

### `agent_kill`

Tool call:

```json
{ "agent_id": "<agent_id>" }
```

Prompt:

```text
Use agent_kill to mark an agent as killed, then confirm via agent_list that its status changed.
```

### `task_post`

Post a task into the shared task board.

Tool call:

```json
{ "title": "Demo task", "description": "Write notes/task.md with a short checklist." }
```

Prompt:

```text
Use task_post to create a Demo task, then call task_list and write notes/tasks.json.
```

### `task_list`

Tool call:

```json
{ "status": "pending" }
```

Prompt:

```text
Use task_list to list pending tasks and write notes/tasks_pending.json.
```

### `task_claim`

Tool call:

```json
{ "agent_id": "<agent_id>" }
```

Prompt:

```text
Use task_post to create a task, then call task_claim to claim the next pending task (optionally pass agent_id). Save the returned claimed task JSON, then call task_complete with its task_id.
```

### `task_complete`

Tool call:

```json
{ "task_id": "<task_id>", "result": "done" }
```

Prompt:

```text
Use task_complete to mark a task completed with a short result string, then verify via task_list.
```

### `event_publish`

Append an event record into the shared event log.

Tool call:

```json
{ "event_type": "demo.finished", "payload": { "ok": true } }
```

Prompt:

```text
Use event_publish to publish a demo.finished event with payload {ok:true}. Then write notes/event_done.md describing what you published.
```

### `schedule_create`

Store a schedule record (definition only; execution depends on your runner/daemon setup).

Tool call:

```json
{ "description": "Daily standup reminder", "schedule": "every day 09:30", "enabled": true }
```

Prompt:

```text
Use schedule_create to create a daily reminder schedule, then call schedule_list and write notes/schedules.json.
```

### `schedule_list`

Tool call:

```json
{}
```

Prompt:

```text
Use schedule_list and write notes/schedules.json with the output.
```

### `schedule_delete`

Tool call:

```json
{ "id": "<schedule_id>" }
```

Prompt:

```text
Use schedule_create, then schedule_delete the returned id, then confirm it no longer appears in schedule_list.
```

### `cron_create`

Store a cron job definition (definition only; execution depends on your runner/daemon setup).

Tool call:

```json
{
  "name": "demo",
  "schedule": "*/5 * * * *",
  "action": "channel_send",
  "delivery": { "channel": "console", "recipient": "stdout", "message": "tick" },
  "one_shot": false
}
```

Prompt:

```text
Use cron_create to store a demo cron definition, then cron_list and write notes/cron.json. (This example stores the definition; it does not automatically run unless you have a runner.)
```

### `cron_list`

Tool call:

```json
{}
```

Prompt:

```text
Use cron_list and write notes/cron.json with the output.
```

### `cron_cancel`

Tool call:

```json
{ "job_id": "<job_id>" }
```

Prompt:

```text
Use cron_create, then cron_cancel the returned job_id, then confirm it no longer appears in cron_list.
```

### `knowledge_add_entity`

Add an entity record to the knowledge store.

Tool call:

```json
{ "name": "RexOS", "entity_type": "project", "properties": { "repo": "rexleimo/rexos" } }
```

Prompt:

```text
Use knowledge_add_entity to add an entity for RexOS, then call knowledge_query for \"RexOS\" and write notes/knowledge.json with the result.
```

### `knowledge_add_relation`

Add a relation record (edge) between two entities.

Tool call:

```json
{
  "source": "RexOS",
  "relation": "inspires",
  "target": "meos",
  "properties": { "confidence": 0.8 }
}
```

Prompt:

```text
Use knowledge_add_relation to relate RexOS -> meos, then query for RexOS.
```

### `knowledge_query`

Search entities/relations (best-effort substring query).

Tool call:

```json
{ "query": "RexOS" }
```

Prompt:

```text
Use knowledge_query for RexOS and write notes/knowledge.json with the JSON output.
```

## `channel_send`

Enqueue an outbound message into the outbox. Delivery happens out-of-band via the dispatcher:

- run once: `loopforge channel drain`
- long-running: `loopforge channel worker`

Supported channels:

- `console`: prints the message on drain
- `webhook`: posts JSON to `REXOS_WEBHOOK_URL`

Arguments (tool call JSON):

- `channel` (required): `console` | `webhook`
- `recipient` (required): for `console`, use something like `"stdout"`; for `webhook`, this can be a logical name (the URL is configured out-of-band)
- `subject` (optional)
- `message` (required)

### Example

Tool call:

```json
{ "channel": "console", "recipient": "stdout", "subject": "demo", "message": "Hello from RexOS" }
```

Prompt:

```text
Use channel_send to enqueue a console message (recipient=stdout) saying \"Hello from RexOS\". Then tell me to run `loopforge channel drain` to deliver it.
```

## `workflow_run`

Run a multi-step workflow and persist execution state to `.rexos/workflows/<workflow_id>.json`.

Arguments (tool call JSON):

- `workflow_id` (optional): stable id for repeatable runs.
- `name` (optional): human-readable workflow name.
- `steps` (required): array of step objects.
  - `tool` (required)
  - `arguments` (optional object; defaults to `{}`)
  - `name` (optional)
  - `approval_required` (optional boolean): force approval gate when approval mode is enabled.
- `continue_on_error` (optional): continue after failed steps.

### Example

Tool call:

```json
{
  "workflow_id": "wf_demo",
  "name": "write-note",
  "steps": [
    {
      "name": "write",
      "tool": "fs_write",
      "arguments": { "path": "notes/workflow.txt", "content": "hello" }
    }
  ]
}
```

Prompt:

```text
Use workflow_run to execute one step that writes notes/workflow.txt with \"hello\", then report workflow status.
```

## `hand_*`

Hands are small, curated “agent templates” that spawn a specialized agent instance.

- `hand_list`: list built-in Hands and whether they are active.
- `hand_activate`: activates a Hand and returns `{instance_id, agent_id, ...}`.
- `hand_status`: returns the current active instance (if any) for a `hand_id`.
- `hand_deactivate`: deactivates a Hand instance by `instance_id` (kills its underlying agent).

After `hand_activate`, you can use `agent_send` to talk to the returned `agent_id`.

### `hand_list`

Tool call:

```json
{}
```

Prompt:

```text
Use hand_list and write notes/hands.json with the output. Pick one available hand id.
```

### `hand_activate`

Tool call:

```json
{ "hand_id": "researcher", "config": { "topic": "RexOS" } }
```

Prompt:

```text
Use hand_activate to activate the researcher hand. Then use agent_send with the returned agent_id to ask it to do a web_search for \"RexOS\" and summarize 3 bullets.
```

### `hand_status`

Tool call:

```json
{ "hand_id": "researcher" }
```

Prompt:

```text
Use hand_status to check if the researcher hand is active, and write notes/hand_status.json with the output.
```

### `hand_deactivate`

Tool call:

```json
{ "instance_id": "<instance_id>" }
```

Prompt:

```text
Use hand_activate to start a hand, then hand_deactivate using the returned instance_id. Confirm via hand_list that it is no longer active.
```

## `a2a_*`

A2A tools let RexOS talk to external A2A-compatible agents:

- `a2a_discover`: fetches the agent card at `/.well-known/agent.json`
- `a2a_send`: sends a JSON-RPC `tasks/send` request to an A2A endpoint URL

Both are SSRF-protected by default; for local testing you can set `allow_private=true`.

### `a2a_discover`

Fetch an A2A agent card (RexOS always requests `/.well-known/agent.json` on the given host).

Tool call:

```json
{ "url": "https://example.com", "allow_private": false }
```

Prompt:

```text
Use a2a_discover on a known A2A host and write notes/agent_card.json with the output.
```

### `a2a_send`

Send a message to an A2A endpoint URL (JSON-RPC `tasks/send`).

Tool call:

```json
{ "agent_url": "http://127.0.0.1:8787/a2a", "message": "hello", "session_id": "demo", "allow_private": true }
```

Prompt:

```text
Use a2a_send to talk to an A2A endpoint and save the returned result JSON to notes/a2a_result.json.
```

## `speech_to_text`

Transcribe media into text.

MVP behavior: supports **text transcript files** (`.txt`, `.md`, `.srt`, `.vtt`) and returns JSON with `transcript` and `text`.

### Example

Tool call:

```json
{ "path": "samples/transcript.txt" }
```

Prompt:

```text
Use fs_write to create samples/transcript.txt with a short transcript. Then call speech_to_text on it and write notes/stt.json with the returned JSON.
```

## `text_to_speech`

Convert text into an audio file.

MVP behavior: writes a short `.wav` file to the workspace (placeholder for real TTS).

### Example

Tool call:

```json
{ "text": "Hello from RexOS", "path": ".rexos/audio/tts.wav" }
```

Prompt:

```text
Use text_to_speech to write .rexos/audio/tts.wav saying \"Hello from RexOS\". Then use media_describe on that file and write notes/tts_meta.json.
```

## `docker_exec`

Run a command inside a one-shot Docker container with the workspace mounted.

- Disabled by default: set `REXOS_DOCKER_EXEC_ENABLED=1`
- Optional image override: `REXOS_DOCKER_EXEC_IMAGE` (default `alpine:3.20`)

### Example

Tool call:

```json
{ "command": "echo hello-from-docker && ls -la" }
```

Prompt:

```text
If you enabled docker_exec, use docker_exec to run a safe command in a container and write notes/docker_exec.json with exit_code/stdout/stderr.
```

## `process_*`

Start and interact with long-running processes:

- `process_start` / `process_poll` / `process_write` / `process_kill` / `process_list`

Processes run with the workspace as the working directory and a minimal environment.

`process_poll` returns JSON:

- `stdout` / `stderr` (incremental)
- `stdout_truncated` / `stderr_truncated` (bool; when true, the output contains a head+tail snippet with `[...] middle omitted [...]`)
- `exit_code` (null while alive)
- `alive` (bool)

### `process_start`

Start a long-running process and return a `process_id`.

Tool call (macOS/Linux example):

```json
{ "command": "bash", "args": ["-lc", "echo READY; read line; echo ECHO:$line; sleep 30"] }
```

Prompt:

```text
Use process_start to start a process that prints READY and then echoes one line. Capture the returned process_id for later steps.
```

### `process_poll`

Tool call:

```json
{ "process_id": "<process_id>" }
```

Prompt:

```text
Use process_poll in a short loop until stdout contains READY. Then continue.
```

### `process_write`

Tool call:

```json
{ "process_id": "<process_id>", "data": "hi" }
```

Prompt:

```text
After READY, use process_write to send \"hi\". Then poll again until you see ECHO:hi.
```

### `process_list`

Tool call:

```json
{}
```

Prompt:

```text
Use process_list and write notes/processes.json with the output (verify your process_id is present).
```

### `process_kill`

Tool call:

```json
{ "process_id": "<process_id>" }
```

Prompt:

```text
Use process_kill to stop the process. Then call process_list to confirm it is gone.
```

## `canvas_present`

Save sanitized HTML to the workspace (under `output/`) and return metadata (`saved_to`, `canvas_id`, ...).

Scripts, event handlers (e.g. `onclick=`), and `javascript:` URLs are rejected.

### Example

Tool call:

```json
{ "title": "Demo report", "html": "<h1>Hello</h1><p>Generated by RexOS.</p>" }
```

Prompt:

```text
Use canvas_present to generate a small HTML report with a title and 3 bullet points. Then fs_read the saved_to path and write notes/report_path.txt with that filename.
```
