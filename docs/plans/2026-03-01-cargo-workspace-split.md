# Cargo Workspace Split Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Convert RexOS from a single crate into an OpenFang-style Cargo workspace with modular crates, while keeping the `rexos` CLI behavior and existing integration tests working.

**Architecture:** Use a virtual workspace root (`Cargo.toml` with `[workspace]`). Move code into `crates/*` and introduce a small `rexos` facade crate that re-exports the public API (`rexos::agent`, `rexos::llm`, `rexos::memory`, etc.). Keep the CLI as `crates/rexos-cli` (binary name: `rexos`) that depends on the facade.

**Tech Stack:** Rust 2021, tokio, reqwest, axum, rusqlite (bundled), clap, serde/toml/serde_json.

---

### Task 1: Create workspace root + crate skeletons

**Files:**
- Modify: `Cargo.toml` (becomes virtual workspace)
- Modify: `init.sh` (run the built `rexos` binary for smoke checks)
- Modify: `README.md` (update dev commands)
- Create: `crates/rexos/Cargo.toml`
- Create: `crates/rexos/src/lib.rs`
- Create: `crates/rexos-cli/Cargo.toml`
- Create: `crates/rexos-cli/src/main.rs`
- Create: `crates/rexos-kernel/Cargo.toml`
- Create: `crates/rexos-kernel/src/lib.rs`
- Create: `crates/rexos-llm/Cargo.toml`
- Create: `crates/rexos-llm/src/lib.rs`
- Create: `crates/rexos-memory/Cargo.toml`
- Create: `crates/rexos-memory/src/lib.rs`
- Create: `crates/rexos-runtime/Cargo.toml`
- Create: `crates/rexos-runtime/src/lib.rs`
- Create: `crates/rexos-tools/Cargo.toml`
- Create: `crates/rexos-tools/src/lib.rs`
- Create: `crates/rexos-harness/Cargo.toml`
- Create: `crates/rexos-harness/src/lib.rs`
- Create: `crates/rexos-daemon/Cargo.toml`
- Create: `crates/rexos-daemon/src/lib.rs`
- (Optional) Create: `crates/rexos-types/*` if extracting common OpenAI-compat types

**Step 1: Verify baseline**

Run: `cargo test`
Expected: PASS

**Step 2: Replace root Cargo.toml with a workspace**

Create a virtual workspace with members for all crates. Use `[workspace.package]` and `[workspace.dependencies]` to DRY versions.

**Step 3: Scaffold crates**

Each crate gets a minimal `lib.rs` that either owns code (moved modules) or re-exports.

**Step 4: Update `init.sh` + README**

- `init.sh` should run the built binary for the smoke check: `./target/debug/rexos --help`.
- README examples should use `rexos ...` (or `./target/<profile>/rexos ...` if not installed).

---

### Task 2: Move code into crates and wire dependencies

**Files:**
- Move: `src/config.rs`, `src/paths.rs`, `src/router/mod.rs` → `crates/rexos-kernel/src/`
- Move: `src/llm/*` → `crates/rexos-llm/src/`
- Move: `src/memory/mod.rs` → `crates/rexos-memory/src/`
- Move: `src/tools/mod.rs` → `crates/rexos-tools/src/`
- Move: `src/agent/mod.rs` → `crates/rexos-runtime/src/`
- Move: `src/harness/mod.rs` → `crates/rexos-harness/src/`
- Move: `src/daemon/mod.rs` → `crates/rexos-daemon/src/`
- Move: `src/main.rs` → `crates/rexos-cli/src/main.rs`
- Replace: `src/lib.rs` → delete (root no longer a package)

**Step 1: Wire module paths**

- Convert `crate::...` imports to the new crate boundaries (e.g. runtime depends on kernel + llm + tools + memory).
- Keep the external API stable by re-exporting through `crates/rexos`.

**Step 2: Ensure the CLI still compiles**

Run: `cargo build -p rexos-cli`
Expected: PASS

---

### Task 3: Move integration tests and verify

**Files:**
- Move: `tests/*` → `crates/rexos/tests/*` (tests continue importing `rexos::...`)

**Step 1: Run full test suite**

Run: `cargo test`
Expected: PASS

**Step 2: Optional local Ollama smoke**

Run: `REXOS_OLLAMA_MODEL=<your-model> cargo test -p rexos -- --ignored`
Expected: PASS (requires local Ollama model)

**Step 3: Commit**

Run:
```bash
git add -A
git commit -m "refactor: split rexos into cargo workspace"
```
