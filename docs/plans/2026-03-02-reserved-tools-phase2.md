# Reserved Tools (Phase 2) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Implement the remaining “reserved stub” tools in RexOS so they no longer return `tool not implemented yet: <name>`: `hand_*`, `a2a_*`, `speech_to_text`, `text_to_speech`, `docker_exec`, `process_*`, `canvas_present`.

**Architecture:**
- `hand_*` are **runtime tools** (like `agent_*`) because they spawn/track specialized agents and should persist in `~/.rexos/rexos.db`.
- Everything else is implemented in `rexos-tools` (`Toolset`) with the existing sandbox rules (workspace-relative paths, SSRF guards, timeouts).
- Keep compatibility-oriented parameter schemas close to the de-facto conventions used by other agent frameworks.

**Tech Stack:** Rust 2021, tokio, reqwest, axum (tests), SQLite KV store (`rexos-memory`).

---

### Task 1: `hand_*` (runtime tools) — RED

**Files:**
- Create: `crates/rexos/tests/reserved_hand_tools.rs`

**Test goals:**
- `hand_list` returns the built-in hands list.
- `hand_activate` returns `{instance_id, agent_id, hand_id, status}` and creates an underlying agent record.
- `hand_status` returns status for a `hand_id`.
- `hand_deactivate` deactivates an instance and kills the underlying agent.

**Run:** `cargo test -p rexos reserved_hand_tools`
**Expected:** FAIL with `tool not implemented yet: hand_*` (or unknown tool).

---

### Task 2: `hand_*` (runtime tools) — GREEN

**Files:**
- Modify: `crates/rexos-runtime/src/lib.rs`
- Modify: `crates/rexos-tools/src/lib.rs` (definitions + Toolset behavior)

**Implementation notes:**
- Add runtime tool handling for: `hand_list`, `hand_activate`, `hand_status`, `hand_deactivate`.
- Persist instances in KV:
  - `rexos.hands.instances.index` → JSON array of `instance_id`
  - `rexos.hands.instances.<instance_id>` → JSON record
- Underlying agent is created via existing `agent_spawn` logic (agent_id = instance_id).
- Update `Toolset::call()` to report `hand_*` as “implemented in runtime”.
- Update tool definitions with real schemas.

**Run:** `cargo test -p rexos reserved_hand_tools`
**Expected:** PASS.

---

### Task 3: `a2a_*` (Toolset) — RED → GREEN

**Files:**
- Modify: `crates/rexos-tools/src/lib.rs`

**Tests (add to `rexos-tools` unit tests):**
- loopback denied by default for `a2a_discover`
- allow loopback when `allow_private=true` and fetch `/.well-known/agent.json`
- `a2a_send` posts JSON-RPC `tasks/send` and returns the `result`

**Run:** `cargo test -p rexos-tools a2a_`
**Expected:** FAIL → then PASS after implementation.

---

### Task 4: `speech_to_text` (Toolset) — RED → GREEN

**Files:**
- Modify: `crates/rexos-tools/src/lib.rs`

**Behavior (MVP):**
- For `.txt/.md/.srt/.vtt`, delegate to existing transcript reader.
- Return JSON with both `text` and `transcript` keys for compatibility.

**Run:** `cargo test -p rexos-tools speech_to_text`

---

### Task 5: `text_to_speech` (Toolset) — RED → GREEN

**Files:**
- Modify: `crates/rexos-tools/src/lib.rs`

**Behavior (MVP):**
- Write a small valid `.wav` file to a workspace-relative `path` (default `.rexos/audio/tts.wav`).
- Return JSON `{path, format, bytes, note}`.

**Run:** `cargo test -p rexos-tools text_to_speech`

---

### Task 6: `docker_exec` (Toolset) — RED → GREEN

**Files:**
- Modify: `crates/rexos-tools/src/lib.rs`
- Modify: `docs-site/reference/tools.md` + `docs-site/zh/reference/tools.md`

**Behavior (MVP, safe-by-default):**
- Disabled unless `REXOS_DOCKER_EXEC_ENABLED=1`.
- When enabled, requires `docker` in PATH; runs a one-shot container command (best-effort) with workspace mounted.

**Run:** `cargo test -p rexos-tools docker_exec`

---

### Task 7: `process_*` (Toolset) — RED → GREEN

**Files:**
- Modify: `crates/rexos-tools/src/lib.rs`

**Behavior (MVP):**
- `process_start`: spawn a child process with piped stdio, store in Toolset state; max 5 processes.
- `process_poll`: drain buffered stdout/stderr since last poll.
- `process_write`: write to stdin (append newline if missing).
- `process_kill`: terminate and remove.
- `process_list`: list processes with `{process_id, command, args, alive}`.

**Run:** `cargo test -p rexos-tools process_`

---

### Task 8: `canvas_present` (Toolset) — RED → GREEN

**Files:**
- Modify: `crates/rexos-tools/src/lib.rs`

**Behavior (MVP):**
- Sanitize HTML (reject `<script>`, event handlers, `javascript:` URLs).
- Save as a full HTML document to `output/canvas_<id>.html` (workspace-relative).
- Return JSON `{canvas_id, title, saved_to, size_bytes}`.

**Run:** `cargo test -p rexos-tools canvas_present`

---

### Task 9: Docs + Verification + Commit

**Files:**
- Modify: `docs-site/reference/tools.md`
- Modify: `docs-site/zh/reference/tools.md`

**Verification:**
- `cargo test`
- `python3 -m mkdocs build --strict` (if docs deps installed)

**Commits (suggested):**
- `feat(runtime): implement hand tools`
- `feat(tools): add a2a tools`
- `feat(tools): add speech tools`
- `feat(tools): add docker_exec`
- `feat(tools): add process tools`
- `feat(tools): add canvas_present`
- `docs: update tools reference`

