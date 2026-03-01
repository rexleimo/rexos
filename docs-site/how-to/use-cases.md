# Common Use Cases (Recipes)

This page is intentionally **practical**: copy-paste commands, expected artifacts, and “what to do next”.

## Pick the right mode

- `rexos agent run`: one-off tasks inside a workspace sandbox (you review/commit the changes).
- `rexos harness init/run`: long tasks with **verification + checkpoints** (recommended for “keep iterating until X passes”).
- `rexos daemon start`: minimal HTTP daemon (currently only `/healthz`) for integration/readiness checks.

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

See `how-to/providers.md` for full provider examples (GLM/MiniMax native included).

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
