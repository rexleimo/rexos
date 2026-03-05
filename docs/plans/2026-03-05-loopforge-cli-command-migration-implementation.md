# LoopForge CLI Command Migration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make `loopforge` the primary CLI command while keeping `rexos` fully compatible during migration.

**Architecture:** Add a second CLI binary entry (`loopforge`) that points to the same `main.rs`, then switch CLI help/branding to LoopForge-first wording. Keep `rexos` command working as a compatibility alias in binaries, docs, and release artifacts until a later deprecation phase. Update high-traffic docs first, then sweep remaining references with explicit compatibility notes.

**Tech Stack:** Rust (`clap` derive CLI), Cargo multi-bin packaging, Python release tooling, GitHub Actions, MkDocs.

---

### Task 1: Lock migration contract with failing CLI tests

**Files:**
- Modify: `crates/rexos-cli/src/main.rs`
- Test: `crates/rexos-cli/src/main.rs`

**Step 1: Add a failing test for primary CLI name**

Add a test asserting Clap command name is `loopforge`:

```rust
#[test]
fn cli_primary_name_is_loopforge() {
    use clap::CommandFactory;
    assert_eq!(Cli::command().get_name(), "loopforge");
}
```

**Step 2: Run targeted test to confirm fail-before-change**

Run: `cargo test -p rexos-cli cli_primary_name_is_loopforge`
Expected: FAIL (`left: "rexos"` vs `right: "loopforge"`).

**Step 3: Add a parse test for compatibility binary name**

Add:

```rust
#[test]
fn cli_parses_config_validate_with_loopforge_binary_name() {
    let parsed = Cli::try_parse_from(["loopforge", "config", "validate"]);
    assert!(parsed.is_ok(), "expected `loopforge config validate` to parse, got: {parsed:?}");
}
```

**Step 4: Run both tests**

Run:
- `cargo test -p rexos-cli cli_primary_name_is_loopforge`
- `cargo test -p rexos-cli cli_parses_config_validate_with_loopforge_binary_name`
Expected: first FAIL, second PASS (documenting current parser behavior).

**Step 5: Commit**

```bash
git add crates/rexos-cli/src/main.rs
git commit -m "test(cli): lock loopforge primary command contract"
```

### Task 2: Add dual binaries and switch CLI branding to LoopForge-first

**Files:**
- Modify: `crates/rexos-cli/Cargo.toml`
- Modify: `crates/rexos-cli/src/main.rs`
- Modify: `crates/rexos-cli/src/doctor.rs`
- Test: `crates/rexos-cli/src/main.rs`

**Step 1: Add `loopforge` binary entry**

In `crates/rexos-cli/Cargo.toml`, keep existing bin and add:

```toml
[[bin]]
name = "loopforge"
path = "src/main.rs"

[[bin]]
name = "rexos"
path = "src/main.rs"
```

**Step 2: Make Clap primary name LoopForge**

Update CLI derive attributes in `main.rs`:
- `#[command(name = "loopforge")]`
- about text to LoopForge wording.

**Step 3: Update operator-facing text**

Update user-visible strings:
- `eprintln!("[rexos] session_id=...")` -> `eprintln!("[loopforge] session_id=...")`
- Doctor hints `run rexos init` -> `run loopforge init (compat: rexos init)`.

**Step 4: Update parser test expectations**

Adjust assertion messages to LoopForge-first wording; keep at least one explicit `rexos` parse test for compatibility.

**Step 5: Run CLI test suite**

Run: `cargo test -p rexos-cli`
Expected: PASS.

**Step 6: Commit**

```bash
git add crates/rexos-cli/Cargo.toml crates/rexos-cli/src/main.rs crates/rexos-cli/src/doctor.rs
git commit -m "feat(cli): add loopforge binary and keep rexos compatibility"
```

### Task 3: Update release packaging to publish LoopForge command without breaking rexos users

**Files:**
- Modify: `scripts/package_release.py`
- Modify: `.github/workflows/release.yml`
- Modify: `.github/workflows/release-dry-run.yml`
- Test: `scripts/tests/test_ci_workflows.py`
- Create: `scripts/tests/test_package_release.py`

**Step 1: Add failing packaging test for dual binary stage**

Create `scripts/tests/test_package_release.py` that stages a fake release and asserts:
- archive base name uses `loopforge-<version>-<target>`
- archive contains `loopforge` binary
- archive also contains `rexos` compatibility binary.

**Step 2: Run new test and confirm failure**

Run: `python3 -m unittest scripts.tests.test_package_release`
Expected: FAIL (current script only packages one binary with `rexos-*` base name).

**Step 3: Extend packager for compatibility binary**

Add script args:
- `--compat-bin` (optional second binary path)
- rename base archive to `loopforge-...`
- copy primary and compatibility binaries into stage dir.

**Step 4: Update release workflows**

Change workflow matrix and smoke tests to use:
- primary bin path: `target/release/loopforge` (or `.exe`)
- smoke command executes `loopforge --help`
- optional compatibility smoke executes `rexos --help` from unpacked archive.

**Step 5: Run scripts tests**

Run:
- `python3 -m unittest scripts.tests.test_package_release`
- `python3 -m unittest scripts.tests.test_ci_workflows`
Expected: PASS.

**Step 6: Commit**

```bash
git add scripts/package_release.py scripts/tests/test_package_release.py scripts/tests/test_ci_workflows.py .github/workflows/release.yml .github/workflows/release-dry-run.yml
git commit -m "build(release): package loopforge as primary with rexos compatibility binary"
```

### Task 4: Migrate docs to LoopForge-first command examples

**Files:**
- Modify: `README.md`
- Modify: `README.zh-CN.md`
- Modify: `docs-site/reference/cli.md`
- Modify: `docs-site/reference/config.md`
- Modify: `docs-site/zh-CN/reference/cli.md`
- Modify: `docs-site/zh-CN/reference/config.md`
- Modify: `docs-site/tutorials/quickstart-ollama.md`
- Modify: `docs-site/zh-CN/tutorials/quickstart-ollama.md`
- Modify: `docs-site/how-to/faq.md`
- Modify: `docs-site/zh-CN/how-to/faq.md`

**Step 1: Add failing docs assertion test**

Extend `scripts/tests/test_ci_workflows.py` or add a new docs-lint test to assert:
- critical docs contain `loopforge init`
- compatibility note still mentions `rexos init`.

**Step 2: Run docs-lint test to confirm failure before edits**

Run: `python3 -m unittest scripts.tests.test_ci_workflows`
Expected: FAIL on missing `loopforge` command references.

**Step 3: Update command snippets**

In high-traffic docs, replace example commands:
- `rexos init` -> `loopforge init`
- `rexos agent run` -> `loopforge agent run`
- keep compatibility callout nearby: "`rexos` still works during migration".

**Step 4: Update wording for binary distribution**

Change "single binary: rexos" to:
- primary binary `loopforge`
- compatibility binary `rexos` during transition.

**Step 5: Run docs + test verification**

Run:
- `python3 -m unittest scripts.tests.test_ci_workflows`
- `python3 -m mkdocs build --strict`
Expected: PASS.

**Step 6: Commit**

```bash
git add README.md README.zh-CN.md docs-site/reference/cli.md docs-site/reference/config.md docs-site/zh-CN/reference/cli.md docs-site/zh-CN/reference/config.md docs-site/tutorials/quickstart-ollama.md docs-site/zh-CN/tutorials/quickstart-ollama.md docs-site/how-to/faq.md docs-site/zh-CN/how-to/faq.md scripts/tests/test_ci_workflows.py
git commit -m "docs: switch examples to loopforge command with rexos compatibility notes"
```

### Task 5: Sweep remaining command references and keep compatibility guardrails

**Files:**
- Modify: `docs-site/**/*.md` (remaining command examples)
- Modify: `init.sh`
- Modify: `crates/rexos-cli/src/main.rs` (optional deprecation notice scaffold)
- Test: `crates/rexos-cli/src/main.rs`

**Step 1: Inventory remaining references**

Run: `rg -n "\\brexos\\b" -S docs-site README.md README.zh-CN.md crates/rexos-cli/src/main.rs init.sh`
Expected: remaining hits are either compatibility notes or legacy internals.

**Step 2: Normalize `init.sh` smoke command**

Change local smoke:
- `./target/debug/rexos --help` -> `./target/debug/loopforge --help`
- optionally add compatibility smoke: `./target/debug/rexos --help`.

**Step 3: Add optional compatibility notice hook (non-blocking)**

Add a guarded env-based message (`LOOPFORGE_SHOW_REXOS_COMPAT_NOTICE=1`) when invoked as `rexos`, to prepare future deprecation communication without noisy defaults.

**Step 4: Run targeted tests and script smoke**

Run:
- `cargo test -p rexos-cli`
- `./init.sh`
Expected: PASS.

**Step 5: Commit**

```bash
git add init.sh crates/rexos-cli/src/main.rs docs-site
git commit -m "chore: finalize loopforge command migration sweep"
```

### Task 6: Full verification and rollout checklist

**Files:**
- N/A (verification + release readiness)

**Step 1: Rust verification**

Run: `cargo test --workspace --locked`
Expected: PASS.

**Step 2: Python/scripts verification**

Run:

```bash
python3 -m unittest \
  scripts.tests.test_ci_workflows \
  scripts.tests.test_verify_version_changelog \
  scripts.tests.test_verify_release_consistency \
  scripts.tests.test_provider_health_report \
  scripts.tests.test_package_release
```

Expected: PASS.

**Step 3: Docs verification**

Run: `python3 -m mkdocs build --strict`
Expected: PASS.

**Step 4: Binary smoke verification**

Run:

```bash
cargo build --release -p rexos-cli --locked
./target/release/loopforge --help >/dev/null
./target/release/rexos --help >/dev/null
```

Expected: both commands work.

**Step 5: Rollout note**

Publish migration note:
- "LoopForge command is now primary."
- "rexos remains supported during transition."
- include deprecation date only when team approves.
