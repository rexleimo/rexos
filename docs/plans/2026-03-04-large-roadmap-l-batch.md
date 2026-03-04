# Large Roadmap Batch (L1/L2/L3/L4) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Deliver the remaining large-scope roadmap capabilities in one coordinated batch: ACP-like durable events/checkpoints, workflow execution runtime, dangerous-action approvals, and GUI browser sandbox onboarding.

**Architecture:** Extend `rexos-runtime` as the durable control plane (events/checkpoints/approvals/workflows), keep `rexos-tools` as schema surface, and expose operational visibility through `rexos-cli`. GUI sandbox remains opt-in via Docker + remote CDP docs/scripts.

**Tech Stack:** Rust (`tokio`, `serde_json`, `clap`), workspace tests, MkDocs docs, Docker Compose.

---

### Task 1: ACP events + delivery checkpoints

**Files:**
- Modify: `crates/rexos-runtime/src/lib.rs`
- Modify: `crates/rexos-cli/src/main.rs`
- Test: `crates/rexos/tests/runtime_controls.rs`

**Step 1: Write failing tests**
- Add test that runs a session and asserts ACP events persisted (`rexos.acp.events`).
- Add test that enqueues + drains channel message and asserts ACP delivery checkpoint persisted per session.

**Step 2: Implement**
- Add runtime ACP event append/list helpers with bounded retention.
- Add checkpoint upsert/read helpers.
- Emit events in `run_session` (session start/end + tool call outcomes).
- Tag outbox records with session id and persist checkpoint on successful delivery.
- Add CLI inspection commands for ACP events/checkpoints.

**Step 3: Verify**
- `cargo test -p rexos --test runtime_controls -- --nocapture`
- `cargo test -p rexos-cli -- --nocapture`

---

### Task 2: Workflow runtime (`workflow_run`)

**Files:**
- Modify: `crates/rexos-tools/src/lib.rs`
- Modify: `crates/rexos-runtime/src/lib.rs`
- Test: `crates/rexos/tests/runtime_controls.rs`

**Step 1: Write failing test**
- Add test that triggers runtime tool `workflow_run` and verifies:
  - workflow state file exists at `.rexos/workflows/<id>.json`
  - steps recorded with success/failure
  - output includes workflow id and final status

**Step 2: Implement**
- Add `workflow_run` tool definition (runtime-owned).
- Add runtime `workflow_run` handler with durable state file writes.
- Execute steps using existing tool path; persist each step result incrementally.

**Step 3: Verify**
- `cargo test -p rexos --test runtime_controls -- --nocapture`

---

### Task 3: Dangerous-action approval policy

**Files:**
- Modify: `crates/rexos-runtime/src/lib.rs`
- Test: `crates/rexos/tests/runtime_controls.rs`

**Step 1: Write failing tests**
- In `enforce` mode, dangerous tool call is blocked with explicit approval error.
- In `warn` mode, same tool call proceeds but records approval warning in ACP event/audit.

**Step 2: Implement**
- Add env-driven policy:
  - `REXOS_APPROVAL_MODE=off|warn|enforce`
  - `REXOS_APPROVAL_ALLOW=<tool1,tool2,all>`
- Enforce policy before tool execution.

**Step 3: Verify**
- `cargo test -p rexos --test runtime_controls -- --nocapture`

---

### Task 4: GUI browser sandbox onboarding (remote CDP)

**Files:**
- Add: `docker/browser-sandbox/README.md`
- Add: `docker/browser-sandbox/docker-compose.yml`
- Add: `scripts/browser_sandbox_up.sh`
- Modify: `docs-site/how-to/browser-automation.md`

**Step 1: Implement artifacts**
- Add a compose stack exposing noVNC + CDP endpoint.
- Add script that boots stack and prints required `REXOS_BROWSER_CDP_HTTP` env.
- Document secure usage and `REXOS_BROWSER_CDP_ALLOW_REMOTE=1` implications.

**Step 2: Verify**
- `bash scripts/browser_sandbox_up.sh --help`
- `python3 -m mkdocs build --strict` (if docs dependencies available)

---

### Task 5: Global verification

Run:
- `cargo test -p rexos --test runtime_controls -- --nocapture`
- `cargo test -p rexos --test channel_dispatcher -- --nocapture`
- `cargo test -p rexos-cli -- --nocapture`
- `cargo test --workspace --locked`

Then commit with one batch commit for this L-phase slice.
