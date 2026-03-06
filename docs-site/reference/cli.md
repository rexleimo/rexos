# CLI Reference

LoopForge ships a single binary: `loopforge`.

## Top-level commands

- `loopforge init` — initialize `~/.loopforge` (config + database)
- `loopforge onboard` — one-command onboarding (`init` + config validate + `doctor` + optional first task)
- `loopforge doctor` — diagnose common setup issues (config, providers, browser, tooling)
- `loopforge agent run` — run a single agent session in a workspace
- `loopforge skills list|show|doctor|run` — discover, inspect, diagnose, and execute local skills
- `loopforge harness init` — initialize a harness workspace (durable artifacts + git)
- `loopforge harness run` — run an incremental harness session
- `loopforge channel drain` / `worker` — send queued outbound notifications
- `loopforge daemon start` — start the HTTP daemon

## `loopforge onboard`

Recommended first command after install:

```bash
loopforge onboard --workspace loopforge-onboard-demo
```

Useful flags:

- `--skip-agent` — only run setup checks, skip the first agent task
- `--starter <hello|workspace-brief|repo-onboarding>` — choose a starter task profile
- `--prompt "..."` — override starter prompts with an explicit first task
- `--timeout-ms <n>` — adjust doctor probe timeout

Behavior:

1. ensures `~/.loopforge` exists
2. validates config
3. runs `loopforge doctor`
4. optionally runs one first agent task
5. for built-in starters, verifies the expected starter artifact was actually created before reporting success
6. writes onboarding reports into the workspace:
   - `.loopforge/onboard-report.json`
   - `.loopforge/onboard-report.md`

The report includes:

- config result
- doctor summary
- suggested next steps
- first-task status
- recommended next command
- starter suggestions

## `loopforge doctor`

Run it anytime you get stuck:

```bash
loopforge doctor
```

Machine-readable output:

```bash
loopforge doctor --json
```

What doctor reports today:

- config/db paths
- config parsing
- router → provider mapping
- missing provider env vars
- local Ollama connectivity (when configured)
- browser prerequisites
- required tooling such as Git

Text output now ends with **Suggested next steps** when there is a likely remediation path.
JSON output keeps `summary` and `checks`, and also includes additive `next_actions` guidance.

## Examples

=== "macOS/Linux"
    ```bash
    loopforge init
    loopforge onboard --workspace loopforge-onboard-demo --starter workspace-brief

    mkdir -p loopforge-work
    loopforge agent run --workspace loopforge-work --prompt "Create hello.txt"

    loopforge doctor --json
    loopforge harness init loopforge-task --prompt "Initialize a features checklist for refactoring this repo"
    loopforge harness run loopforge-task --prompt "Continue"
    ```

=== "Windows (PowerShell)"
    ```powershell
    loopforge init
    loopforge onboard --workspace loopforge-onboard-demo --starter workspace-brief

    mkdir loopforge-work
    loopforge agent run --workspace loopforge-work --prompt "Create hello.txt"

    loopforge doctor --json
    loopforge harness init loopforge-task --prompt "Initialize a features checklist for refactoring this repo"
    loopforge harness run loopforge-task --prompt "Continue"
    ```
