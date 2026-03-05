# Harness Recipes (Checkpoints)

Use the harness when you want LoopForge to iterate until your verifier passes, while staying rollback-friendly.

## 1) Fix a failing test suite with harness checkpoints

**Goal:** keep making changes until your verifier passes (tests, lint, build, smoke checks), while staying rollback-friendly.

### Steps

1) In the repo you want to fix (recommended), initialize the harness:

```bash
cd /path/to/your/repo
loopforge harness init . --prompt "Create a features checklist for: all tests passing, lint clean, and basic smoke check"
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
loopforge harness run . --prompt "Continue. Focus on the next failing verifier output."
```

### What to expect

- Harness creates/uses durable artifacts in the workspace:
  - `features.json` (checklist)
  - `rexos-progress.md` (append-only progress log)
  - `init.sh` + `init.ps1` (your verifier scripts)
- When your verifier passes, LoopForge makes a **checkpoint git commit**.

!!! tip "Rollback-friendly"
    If a checkpoint is bad, use git normally (e.g. `git reset --hard HEAD~1`) and run `loopforge harness run` again.

## 2) Long refactors with checkpoints (keep scope small per run)

Instead of “big bang refactor”, do multiple harness runs, each with a narrow goal:

1) isolate a module
2) update imports
3) fix compilation
4) fix unit tests
5) run the verifier scripts

This keeps diffs reviewable and failures easy to diagnose.

## 3) Share reproducible “agent tasks”

If you commit the harness artifacts (`features.json`, `rexos-progress.md`, init scripts), others can reproduce the same long-task loop (and extend it) without re-inventing the harness.
