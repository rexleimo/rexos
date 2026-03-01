# Browser Tools (Playwright Bridge) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add browser automation tools to RexOS (`browser_navigate/click/type/read_page/screenshot/close`) and document installation + usage. Keep SSRF protections similar to `web_fetch`. Use Ollama for local smoke testing.

**Architecture:** Implement browser tools inside `rexos-tools` as a Python Playwright bridge (JSON-lines over stdin/stdout), lazily spawned and reused per `Toolset` instance (one session per agent run). Enforce URL scheme + DNS resolution + IP allow/deny checks in Rust before navigation. Keep the feature optional: if Playwright isn’t installed, return a clear error with install steps.

**Tech Stack:** Rust (`tokio` + `tokio::process`), embedded Python script (`include_str!`), Playwright (Python), MkDocs docs.

---

### Task 1: Tool definitions include browser tools (RED → GREEN)

**Files:**
- Modify: `crates/rexos-tools/src/lib.rs`
- Test: `crates/rexos-tools/src/lib.rs`

**Step 1: Write the failing test**

Add a unit test asserting that `Toolset::definitions()` contains:
`browser_navigate`, `browser_click`, `browser_type`, `browser_read_page`, `browser_screenshot`, `browser_close`.

**Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p rexos-tools tool_definitions_include_browser_tools
```

Expected: FAIL (tools not present yet).

**Step 3: Implement minimal code**

Add tool definitions and include them in `definitions()`.

**Step 4: Run test to verify it passes**

Run:

```bash
cargo test -p rexos-tools tool_definitions_include_browser_tools
```

Expected: PASS.

---

### Task 2: `browser_navigate` SSRF guard matches `web_fetch` (RED → GREEN)

**Files:**
- Modify: `crates/rexos-tools/src/lib.rs`
- Test: `crates/rexos-tools/src/lib.rs`

**Step 1: Write failing tests**

Add two tests:
- loopback denied by default: `browser_navigate` with `http://127.0.0.1/` returns error mentioning loopback/private
- loopback allowed when `allow_private=true`

**Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p rexos-tools browser_navigate
```

Expected: FAIL (tool not implemented).

**Step 3: Implement minimal SSRF-checked navigate**

Implement `browser_navigate` path that:
- parses URL, requires http/https
- resolves host to IPs
- denies forbidden IPs unless `allow_private=true`
- only after checks, spawns/uses the bridge

**Step 4: Run tests to verify they pass**

Run:

```bash
cargo test -p rexos-tools browser_navigate
```

Expected: PASS (tests use loopback, should not spawn a browser when denied).

---

### Task 3: Add Python bridge + session lifecycle

**Files:**
- Create: `crates/rexos-tools/src/browser_bridge.py`
- Modify: `crates/rexos-tools/src/lib.rs`
- Test: `crates/rexos-tools/src/lib.rs`

**Step 1: Write a failing test for `browser_close` without a session**

Expected: should succeed and return `"ok"` (idempotent close).

**Step 2: Implement session + close**

Add a `BrowserSession` stored in `Toolset` (lazy init, shared via `Arc<Mutex<...>>`), and implement:
- `browser_close` kills the subprocess if present
- `Drop` kills session on exit (best-effort)

**Step 3: Run tests**

Run:

```bash
cargo test -p rexos-tools
```

Expected: PASS.

---

### Task 4: Implement the remaining browser tools (click/type/read/screenshot)

**Files:**
- Modify: `crates/rexos-tools/src/lib.rs`
- Create: `crates/rexos-tools/src/browser_bridge.py`
- Modify: `crates/rexos-tools/Cargo.toml`

**Implementation notes:**
- `browser_read_page` returns JSON with `{title,url,content}` (truncate content)
- `browser_screenshot` decodes base64 PNG and writes to a workspace path (default `.rexos/browser/screenshot.png`)
- `browser_click` / `browser_type` take selectors and return small JSON status

---

### Task 5: Docs + Ollama smoke test instructions

**Files:**
- Modify: `docs-site/explanation/concepts.md`
- Modify: `docs-site/explanation/security.md`
- Modify: `docs-site/how-to/use-cases.md`
- Modify: `docs-site/zh/explanation/concepts.md`
- Modify: `docs-site/zh/explanation/security.md`
- Modify: `docs-site/zh/how-to/use-cases.md`

**Step 1: Document installation**

Add a short “Browser tools require Playwright” section:

```bash
python3 -m pip install playwright
python3 -m playwright install chromium
```

**Step 2: Add one example use case**

Example prompt: navigate → read page → write summary to file.

**Step 3: Verify docs build**

Run:

```bash
python3 -m mkdocs build --strict
```

**Step 4: Ollama smoke test**

Run:

```bash
REXOS_OLLAMA_MODEL=llama3.2 cargo test -p rexos -- --ignored ollama_openai_compat_smoke
```

