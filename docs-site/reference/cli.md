# CLI Reference

LoopForge ships a primary binary: `loopforge`.
Compatibility binary `rexos` remains available during migration.

## Top-level commands

- `loopforge init` — initialize `~/.rexos` (config + database)
- `loopforge onboard` — one-command onboarding (`init` + config validate + `doctor` + optional first task)
- `loopforge doctor` — diagnose common setup issues (config, providers, browser, tooling)
- `loopforge agent run` — run a single agent session in a workspace
- `loopforge channel drain` — drain queued outbox messages once
- `loopforge channel worker` — run a polling outbox dispatcher
- `loopforge acp events` — list recent ACP events (optional session filter)
- `loopforge acp checkpoints` — show delivery checkpoints for a session
- `loopforge harness init` — initialize a harness workspace (durable artifacts + git)
- `loopforge harness run` — run an incremental harness session
- `loopforge daemon start` — start the HTTP daemon

Compatibility note: all commands above also work with `rexos` during migration.

## Examples

=== "macOS/Linux"
    ```bash
    loopforge init
    loopforge onboard --workspace rexos-onboard-demo

    mkdir -p rexos-work
    loopforge agent run --workspace rexos-work --prompt "Create hello.txt"

    mkdir -p rexos-task
    loopforge harness init rexos-task --prompt "Initialize a features checklist for refactoring this repo"
    loopforge harness run rexos-task --prompt "Continue"

    loopforge channel drain
    loopforge acp events --limit 20

    loopforge daemon start --addr 127.0.0.1:8787
    ```

=== "Windows (PowerShell)"
    ```powershell
    loopforge init
    loopforge onboard --workspace rexos-onboard-demo

    mkdir rexos-work
    loopforge agent run --workspace rexos-work --prompt "Create hello.txt"

    mkdir rexos-task
    loopforge harness init rexos-task --prompt "Initialize a features checklist for refactoring this repo"
    loopforge harness run rexos-task --prompt "Continue"

    loopforge channel drain
    loopforge acp events --limit 20

    loopforge daemon start --addr 127.0.0.1:8787
    ```
