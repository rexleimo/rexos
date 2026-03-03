# Competitor Alignment (OpenClaw): Tool Truncation + Browser Diagnostics + PDF v2 Plan

> **For Claude:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Align a few high-signal reliability features from OpenClaw:

1) tool-result truncation that preserves **head + tail** (especially for diagnostics),
2) better Browser/CDP startup diagnostics (stderr tail included),
3) a more OpenClaw-like `pdf` extraction interface (page-range support).

**Architecture:** Keep RexOS’s existing tool surface and semantics:

- `web_fetch` remains a fetch tool (SSRF-protected), but returns a better truncated body.
- `process_*` remains incremental via `process_poll`, but when output bursts exceed the buffer, return head+tail instead of tail-only.
- `browser_*` remains CDP-based; only improve error reporting on startup.
- `pdf` stays extraction-only (no nested LLM calls), but supports a `pages` page-range argument.

**Tech Stack:** Rust (`tokio`, `reqwest`, `axum` tests), `pdf-extract`.

---

## Task 1: `web_fetch` head+tail truncation

**Files:**
- Modify: `crates/rexos-tools/src/lib.rs`
- Test: `crates/rexos-tools/src/lib.rs`

**Step 1: Add failing test**

- Add `web_fetch_truncation_preserves_head_and_tail`:
  - start an Axum local server returning a body like `HEAD + (many As) + TAIL`
  - call `web_fetch` with `allow_private=true` and small `max_bytes`
  - assert returned `body` contains `HEAD`, `TAIL`, and an omission marker

Run: `cargo test -p rexos-tools web_fetch_truncation_preserves_head_and_tail`
Expected: FAIL (tail not preserved today).

**Step 2: Implement**

- When `resp.bytes().len() > max_bytes`, return:
  - head bytes (budgeted)
  - marker
  - tail bytes (budgeted)
- Keep existing JSON shape; add optional `total_bytes` field if useful (non-breaking).

**Step 3: Verify**

Run: `cargo test -p rexos-tools web_fetch_truncation_preserves_head_and_tail`
Expected: PASS.

---

## Task 2: `process_poll` head+tail buffering for burst output

**Files:**
- Modify: `crates/rexos-tools/src/lib.rs`
- Test: `crates/rexos-tools/src/lib.rs`

**Step 1: Add failing test**

- Add `process_poll_truncation_preserves_head_and_tail`:
  - start a process that prints `HEAD_START`, then >200k output, then `TAIL_END`, then sleeps
  - poll until output contains `TAIL_END`
  - assert the returned `stdout` contains both `HEAD_START` and `TAIL_END` (not tail-only), plus marker

Run: `cargo test -p rexos-tools process_poll_truncation_preserves_head_and_tail`
Expected: FAIL before implementation.

**Step 2: Implement**

- Replace the `Vec<u8>` output buffers with a small struct that stores:
  - `head` (fixed cap)
  - `tail` (fixed cap)
  - `total_bytes` (to detect truncation)
- On poll:
  - if `total_bytes <= max_bytes`, reconstruct the full output (de-overlap head/tail) and decode once
  - if `total_bytes > max_bytes`, decode head + marker + decode tail
- Keep poll incremental by resetting the buffer after each poll.

**Step 3: Verify**

Run: `cargo test -p rexos-tools process_poll_truncation_preserves_head_and_tail`
Expected: PASS.

---

## Task 3: Browser/CDP startup diagnostics (stderr tail)

**Files:**
- Modify: `crates/rexos-tools/src/browser_cdp.rs`
- Test: `crates/rexos-tools/src/browser_cdp.rs`

**Step 1: Add failing test**

- Add `read_devtools_url_includes_stderr_tail_on_exit`:
  - spawn a child that writes known lines to stderr and exits
  - call `read_devtools_url(child.stderr)` and assert the error string contains those stderr lines

Run: `cargo test -p rexos-tools read_devtools_url_includes_stderr_tail_on_exit`
Expected: FAIL before implementation.

**Step 2: Implement**

- Capture the last N lines of stderr while waiting for the “DevTools listening on …” line.
- Include the captured tail in both:
  - timeout error
  - early-exit error

**Step 3: Verify**

Run: `cargo test -p rexos-tools read_devtools_url_includes_stderr_tail_on_exit`
Expected: PASS.

---

## Task 4: PDF v2 – page range support

**Files:**
- Modify: `crates/rexos-tools/src/lib.rs`
- Modify: `docs-site/reference/tools.md`
- Modify: `docs-site/zh-CN/reference/tools.md`

**Step 1: Add failing test**

- Add `pdf_pages_range_selects_requested_pages`:
  - create a small PDF fixture with at least 3 pages (or reuse `pdf-extract` fixture strategy)
  - call `pdf` with `pages="2"` and assert the output text does **not** contain page 1 marker text

Run: `cargo test -p rexos-tools pdf_pages_range_selects_requested_pages`
Expected: FAIL before implementation.

**Step 2: Implement**

- Extend `PdfArgs` and JSON schema with `pages` (string).
- Implement OpenClaw-like parsing:
  - `"1-5"`, `"1,3,5-7"`
  - ignore out-of-range pages; reject invalid syntax
- Apply selection after extraction (since `pdf-extract` returns all pages).

**Step 3: Docs**

- Document the `pages` argument (en + zh-CN).

---

## Task 5: Verify + merge

Run:

- `cargo test`
- `python3 -m mkdocs build --strict`

Then merge back to `main`, push, and remove worktree.

