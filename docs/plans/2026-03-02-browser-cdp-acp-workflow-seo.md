# Browser CDP + Session Persistence + Docs/SEO

> **For Claude:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Make RexOS browser automation work out-of-the-box without Python/Playwright by default (native CDP), improve “resume conversation” ergonomics for `rexos agent run`, and polish docs + SEO for https://os.rexai.top.

**Architecture:** Keep the existing `browser_*` tool surface stable, but swap the default implementation from a Python Playwright bridge to a Rust CDP session. Preserve the Python bridge as an optional fallback backend for compatibility. Make sessions persistent per workspace by default via `.rexos/session_id` when `--session` is not provided.

**Tech Stack:** Rust (tokio, reqwest, tokio-tungstenite), MkDocs Material + `mkdocs-static-i18n`, GitHub Pages.

---

## Scope

### In scope

- **Browser tools:** `browser_navigate/click/type/press_key/wait_for/read_page/screenshot/close`
  - Default backend: **native CDP** (no Python dependency).
  - Optional backend: existing **Playwright bridge** (env-configured).
  - Optional remote browser: connect to an existing CDP endpoint (for Docker/remote GUI cases).
- **Security hardening:** keep SSRF guard; ensure `read_page/screenshot` never return content for disallowed targets.
- **Ergonomics:** `rexos agent run` defaults to a stable per-workspace session id.
- **Docs:** update browser docs + use cases to match the new default backend and give “copy/paste” runnable cases.
- **SEO:** improve docs site metadata, social previews, and navigation discoverability.

### Out of scope (separate later work)

- Full ACP runtime implementation (Agent Client Protocol) with DB-backed event logs and delivery checkpoints (we’ll capture design notes + a minimal roadmap only).
- Full Lobster DSL runtime (we’ll plan a RexOS-native equivalent and reserve tool/CLI names).
- A full “GUI browser sandbox image” with noVNC (we’ll add remote-CDP support now, and document how to run a container later).

---

## Task 1: Add CDP browser backend (native)

**Files:**
- Modify: `crates/rexos-tools/Cargo.toml`
- Modify: `crates/rexos-tools/src/lib.rs`
- (Optional) Create: `crates/rexos-tools/src/browser_cdp.rs`
- Test: `crates/rexos-tools/src/lib.rs` (unit tests still use stub bridge)

**Step 1: Add deps needed for CDP WebSocket**

Add:
- `tokio-tungstenite`
- `futures`
- `dashmap`

Run: `cargo test -p rexos-tools`
Expected: PASS (no behavior change yet).

**Step 2: Implement Chromium discovery + CDP session bootstrap**

Implement:
- Chromium executable resolution with:
  - `REXOS_BROWSER_CHROME_PATH` (explicit path)
  - common OS locations + `PATH` fallbacks
- Spawn Chromium with:
  - `--remote-debugging-port=0`
  - `--user-data-dir <temp>`
  - `--headless=new` when headless
  - safe flags (`--no-first-run`, `--no-default-browser-check`, etc.)
- Parse stderr for “DevTools listening on ws://…”
- GET `/json/list` to find the page target `webSocketDebuggerUrl`
- Connect over WS and enable `Page` + `Runtime`

**Step 3: Wire `browser_*` tools to CDP commands**

Implement (best-effort, minimal surface):
- Navigate: `Page.navigate` + wait for load + return `{title,url}`
- Click: `Runtime.evaluate` JS snippet to `querySelector` then text fallback, then click + return `{title,url,...}`
- Type: `Runtime.evaluate` to focus + set `.value` + dispatch input/change
- PressKey:
  - focus selector via JS (if provided)
  - send `Input.dispatchKeyEvent` (`keyDown` + `keyUp`) for common keys (Enter/Tab/Escape/Arrow keys); fallback to JS if unknown
- WaitFor:
  - selector polling (`document.querySelector`) and/or text polling (`document.body.innerText.includes`)
- ReadPage: `Runtime.evaluate` to extract `{title,url,content}` with truncation
- Screenshot: `Page.captureScreenshot` -> write PNG to workspace-relative path
- Close: kill chromium process and drop CDP connection

**Security checks (must keep):**
- Keep SSRF guard on `browser_navigate` (existing).
- Store `allow_private` in session state.
- Re-check the *current* `location.href` on `read_page` and `screenshot`; if forbidden and `allow_private=false`, return an error.

Run: `cargo test -p rexos-tools`
Expected: PASS.

---

## Task 2: Preserve Playwright bridge as optional backend

**Files:**
- Modify: `crates/rexos-tools/src/lib.rs`
- Keep: `crates/rexos-tools/src/browser_bridge.py` (legacy backend)
- Modify: `docs-site/how-to/browser-automation.md`

**Behavior:**
- Add `REXOS_BROWSER_BACKEND=playwright|cdp` (default `cdp`).
- If `playwright`, use existing bridge logic and env vars:
  - `REXOS_BROWSER_PYTHON`
  - `REXOS_BROWSER_BRIDGE_PATH`

Update unit tests that set a stub bridge to also set `REXOS_BROWSER_BACKEND=playwright`.

Run: `cargo test -p rexos-tools`
Expected: PASS.

---

## Task 3: Remote CDP endpoint (for future GUI sandbox)

**Files:**
- Modify: `crates/rexos-tools/src/lib.rs`
- Modify: `docs-site/how-to/browser-automation.md`

Add env:
- `REXOS_BROWSER_CDP_HTTP=http://127.0.0.1:9222` (or similar)

If set:
- do not spawn Chromium locally
- fetch `/json/list` from this endpoint to find a `page` target
- connect WS to that target and run the same tool actions

This enables later adding a Docker/NoVNC “GUI browser sandbox” without changing tool UX.

---

## Task 4: E2E smoke (headed) with Ollama + Baidu weather

**Files:**
- Modify: `crates/rexos/tests/browser_baidu_weather_smoke.rs`
- Modify: `docs-site/how-to/browser-use-cases/baidu-weather.md`

Update test notes to reflect the new default backend (CDP; no Python required).

Run:
`REXOS_OLLAMA_MODEL=qwen3:4b cargo test -p rexos --test browser_baidu_weather_smoke -- --ignored`

Expected:
- A Chromium window appears (when headless=false)
- Tool calls succeed
- Ollama logs show a chat completion request
- Test prints a short Chinese summary and passes

---

## Task 5: “ACP-like” session persistence for `rexos agent run`

**Files:**
- Modify: `crates/rexos-cli/src/main.rs`
- (Optional) Add test: `crates/rexos/tests/agent_session_persistence.rs`
- Docs: `docs-site/tutorials/new-user-walkthrough.md`

Change default behavior:
- If `rexos agent run` is called without `--session`, derive session id from workspace:
  - Use `rexos::harness::resolve_session_id(&workspace)` to create/read `.rexos/session_id`
  - This makes “follow-up prompts” continue the same conversation by default.

Run: `cargo test -p rexos`
Expected: PASS.

---

## Task 6: Lobster-style workflow planning (design + reserved surface)

**Files:**
- Add doc: `docs/plans/2026-03-02-workflow-lobster-style.md` (design)
- Docs: `docs-site/tutorials/harness-long-task.md` (mention future `workflow_run`)

Deliverable:
- A short design doc describing:
  - `workflow_run` tool (YAML steps, approval gates, resume token)
  - durable state storage under `.rexos/workflows/<id>.json`
  - how this complements harness checkpoints

No implementation required in this batch unless requested.

---

## Task 7: Docs SEO + navigation polish

**Files:**
- Modify: `mkdocs.yml`
- Modify: `docs-site/index.md`
- Add: `docs-site/assets/images/og-card.png` (optional)
- Modify: `docs-site/assets/stylesheets/extra.css` (optional)

Checklist:
- Ensure `site_name/site_description/site_url` are accurate (already set).
- Add:
  - favicon + logo
  - stronger landing page copy (keywords: long-running agent, harness, tools, memory, routing)
  - “Use cases” and “Browser” entry points on the homepage
- Verify `sitemap.xml` is generated and includes `site_url` canonical links.

Run: `python -m mkdocs build --strict`
Expected: PASS and updated `site/` output.

---

## Release / Verification

Run:
- `cargo test`
- `python -m mkdocs build --strict`

Commit with 2-4 focused commits (browser backend, docs, CLI persistence, SEO). Push to `origin/main` via PR or fast-forward merge (project preference).

