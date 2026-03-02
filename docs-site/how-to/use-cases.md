# Common Use Cases (Recipes)

This page is intentionally **practical**: copy-paste commands, expected artifacts, and “what to do next”.

## Pick the right mode

- `rexos agent run`: one-off tasks inside a workspace sandbox (you review/commit the changes).
- `rexos harness init/run`: long tasks with **verification + checkpoints** (recommended for “keep iterating until X passes”).
- `rexos daemon start`: minimal HTTP daemon (currently only `/healthz`) for integration/readiness checks.

---

## 0) Setup sanity check (recommended)

**Goal:** verify your local config, model routing, and tool sandbox in ~2 minutes.

### Steps

1) Initialize once:

```bash
rexos init
```

2) Create a scratch workspace:

```bash
mkdir -p rexos-demo
cd rexos-demo
```

3) Run a tiny task that writes files + runs a shell command:

```bash
rexos agent run --workspace . --prompt "Create notes/hello.md with a short greeting. Then run shell command 'pwd && ls -la'. Save the command output to notes/env.txt. End by replying with the paths you wrote."
```

### What to expect

- `notes/hello.md`
- `notes/env.txt`

---

## 1) Fix a failing test suite with harness checkpoints

**Goal:** keep making changes until your verifier passes (tests, lint, build, smoke checks), while staying rollback-friendly.

### Steps

1) In the repo you want to fix (recommended), initialize the harness:

```bash
cd /path/to/your/repo
rexos harness init . --prompt "Create a features checklist for: all tests passing, lint clean, and basic smoke check"
```

2) Customize the init script to reflect your verifier (tests/build/lint).

=== "Bash (macOS/Linux)"
    ```bash
    ./init.sh
    ```

=== "PowerShell (Windows)"
    ```powershell
    .\init.ps1
    ```

3) Run incremental loops until it passes:

```bash
rexos harness run . --prompt "Continue. Focus on the next failing verifier output."
```

### What to expect

- Harness creates/uses durable artifacts in the workspace:
  - `features.json` (checklist)
  - `rexos-progress.md` (append-only progress log)
  - `init.sh` + `init.ps1` (your verifier scripts)
- When your verifier passes, RexOS makes a **checkpoint git commit**.

!!! tip "Rollback-friendly"
    If a checkpoint is bad, use git normally (e.g. `git reset --hard HEAD~1`) and run `rexos harness run` again.

---

## 2) Mechanical edits across a repo (safe workspace sandbox)

Use `agent run` when you want a targeted change and prefer manual review/commit.

```bash
cd /path/to/repo
rexos agent run --workspace . --prompt "Rename Foo to Bar across the codebase. Update imports and keep tests passing."
```

Good prompts for this pattern:

- “Apply this change across all files and run the formatter.”
- “Update deprecated API calls and add a small regression test.”
- “Migrate config format and keep backwards compatibility.”

---

## 3) Route planning locally (Ollama) and coding to a stronger model

This is a common workflow:

- planning: small/local (cheap, fast)
- coding: stronger cloud model
- summary: cheap summarizer

Example routing:

```toml
[router.planning]
provider = "ollama"
model = "default"

[router.coding]
provider = "glm_native" # or minimax_native / deepseek / kimi / qwen_native ...
model = "default"

[router.summary]
provider = "ollama"
model = "default"
```

See `how-to/providers.md` for full provider examples (GLM/MiniMax native + NVIDIA NIM included).

---

## 4) Long refactors with checkpoints (keep scope small per run)

Instead of “big bang refactor”, do multiple harness runs, each with a narrow goal:

1) isolate a module
2) update imports
3) fix compilation
4) fix unit tests
5) run the verifier scripts

This keeps diffs reviewable and failures easy to diagnose.

---

## 5) Share reproducible “agent tasks”

If you commit the harness artifacts (`features.json`, `rexos-progress.md`, init scripts), others can reproduce the same long-task loop (and extend it) without re-inventing the harness.

---

## 6) Daemon mode for readiness checks (experimental)

The daemon currently exposes a simple health endpoint:

```bash
rexos daemon start --addr 127.0.0.1:8787
curl http://127.0.0.1:8787/healthz
```

Use it for container readiness / supervision, and keep the rest of RexOS logic in the CLI for now.

---

## 7) Local testing with small models (recommended)

Validate tool-calling + harness flow with Ollama first, then switch routing to bigger models once the loop is stable.

---

## 8) Browser automation (Playwright bridge)

Use browser tools when you need to interact with dynamic pages (JS-rendered content, clicking, typing, screenshots).

See also: [Browser Automation (Playwright)](browser-automation.md).

### Prerequisites

Install Playwright (Python):

```bash
python3 -m pip install playwright
python3 -m playwright install chromium
```

### Example: open a page, summarize, save artifacts

```bash
rexos agent run --workspace . --prompt "Use browser tools to open https://example.com, read the page, write a short summary to notes/example.md, and save a screenshot to .rexos/browser/example.png."
```

Notes:

- `browser_navigate` is SSRF-checked by default (set `allow_private=true` only for local/private targets).
- Screenshots write to workspace-relative paths (no absolute paths, no `..`, no symlink escapes).

---

## 9) Notifications via `channel_send` (outbox + dispatcher)

`channel_send` enqueues an outbound message into an outbox. Delivery happens when you run the dispatcher:

```bash
rexos channel drain
```

Or run a long-lived worker:

```bash
rexos channel worker --interval-secs 5
```

### Example: send a console notification

```bash
rexos agent run --workspace . --prompt "Use channel_send to enqueue: channel=console recipient=me subject=Hello message=Done"
rexos channel drain
```

### Example: send to a webhook

```bash
export REXOS_WEBHOOK_URL="https://example.com/my-webhook"
rexos agent run --workspace . --prompt "Use channel_send to enqueue: channel=webhook recipient=user1 message=hello"
rexos channel drain
```

---

## 10) Browser demo: GUI screenshot + summary (example.com)

Use this to verify browser automation end-to-end, with **persistent artifacts** in your workspace.

### Steps

1) Install Playwright (Python):

```bash
python3 -m pip install playwright
python3 -m playwright install chromium
```

2) Run the demo (GUI mode):

=== "Bash (macOS/Linux)"
    ```bash
    export REXOS_BROWSER_HEADLESS=0
    rexos agent run --workspace . --prompt "Use browser tools to open https://example.com, read the page, write a 3-bullet summary to notes/example.md, save a screenshot to .rexos/browser/example.png, then close the browser."
    ```

=== "PowerShell (Windows)"
    ```powershell
    $env:REXOS_BROWSER_HEADLESS = "0"
    rexos agent run --workspace . --prompt "Use browser tools to open https://example.com, read the page, write a 3-bullet summary to notes/example.md, save a screenshot to .rexos/browser/example.png, then close the browser."
    ```

### What to expect

- `notes/example.md`
- `.rexos/browser/example.png`

---

## 11) Browser + Ollama: Baidu “today’s weather” (real-world flow)

This is a more “real” flow: open a search results page, extract weather info, and save it.

### Steps

1) Make sure Ollama has an instruction model (example):

```bash
ollama pull qwen3:4b
```

2) (Optional, recommended) Use it as RexOS default model:

Edit `~/.rexos/config.toml` and set:

```toml
[providers.ollama]
default_model = "qwen3:4b"
```

3) Run (GUI mode):

=== "Bash (macOS/Linux)"
    ```bash
    export REXOS_BROWSER_HEADLESS=0
    rexos agent run --workspace . --prompt "Use browser tools to open https://www.baidu.com/s?wd=%E5%8C%97%E4%BA%AC%20%E4%BB%8A%E5%A4%A9%E5%A4%A9%E6%B0%94 . Wait for #content_left, then read the page. Extract today's weather info (conditions, temperature range, wind) from the page text. Write it to notes/weather.md. Save a screenshot to .rexos/browser/baidu_weather.png. Close the browser. If you can't find the weather, say so, but still save the screenshot."
    ```

=== "PowerShell (Windows)"
    ```powershell
    $env:REXOS_BROWSER_HEADLESS = "0"
    rexos agent run --workspace . --prompt "Use browser tools to open https://www.baidu.com/s?wd=%E5%8C%97%E4%BA%AC%20%E4%BB%8A%E5%A4%A9%E5%A4%A9%E6%B0%94 . Wait for #content_left, then read the page. Extract today's weather info (conditions, temperature range, wind) from the page text. Write it to notes/weather.md. Save a screenshot to .rexos/browser/baidu_weather.png. Close the browser. If you can't find the weather, say so, but still save the screenshot."
    ```

### What to expect

- `notes/weather.md`
- `.rexos/browser/baidu_weather.png`

!!! note "If you hit a CAPTCHA"
    Some sites may show CAPTCHAs or block automation. If that happens, try a different query/site, or switch to `web_search` + `web_fetch` when the content is not JS-heavy.

---

## 12) (From source) Run the browser + Ollama smoke test

If you're hacking on RexOS itself, you can run the ignored smoke test:

```bash
REXOS_OLLAMA_MODEL=qwen3:4b cargo test -p rexos --test browser_baidu_weather_smoke -- --ignored --nocapture
```

Expected output includes a line like:

- `[rexos][baidu_weather] summary=...`

This test uses a temp workspace and cleans it up. Use the recipes above if you want to keep screenshots and files.
