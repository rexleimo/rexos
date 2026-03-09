# CLI Reference

LoopForge ships one binary: `loopforge`.

## Which command should I use?

Use this quick map when you are not sure where to start:

- First install or reset: `loopforge init`
- Validate config syntax and schema: `loopforge config validate`
- Diagnose environment readiness: `loopforge doctor`
- Want a guided first-run: `loopforge onboard`
- Run one agent task in a workspace: `loopforge agent run`
- Run longer incremental work with durable workspace artifacts: `loopforge harness init` + `loopforge harness run`
- Inspect or execute local skills: `loopforge skills list|show|doctor|run`
- Deliver queued outbound messages: `loopforge channel drain` / `loopforge channel worker`
- Run stored cron jobs (optional worker): `loopforge cron tick` / `loopforge cron worker`
- Run LoopForge as an HTTP service: `loopforge daemon start`
- Inspect ACP events and checkpoints: `loopforge acp events` / `loopforge acp checkpoints`
- Check release metadata before publishing: `loopforge release check`

## Command families

Top-level commands are organized by job type:

- `loopforge init` — initialize `~/.loopforge` (config + database)
- `loopforge onboard` — one guided setup pass (`init` + config validate + `doctor` + optional first task)
- `loopforge doctor` — diagnose common setup issues (config, providers, browser, tooling)
- `loopforge config validate` — validate `~/.loopforge/config.toml`
- `loopforge agent run` — run a single agent session in a workspace
- `loopforge harness init|run` — initialize and continue long-running harness workspaces
- `loopforge skills list|show|doctor|run` — discover, inspect, diagnose, and execute local skills
- `loopforge channel drain|worker` — deliver queued outbound notifications
- `loopforge cron tick|worker` — run stored cron jobs (optional worker)
- `loopforge daemon start` — start the HTTP daemon
- `loopforge acp events|checkpoints` — inspect ACP events and delivery checkpoints
- `loopforge release check` — verify release metadata and preflight conditions

## Recommended first-run flow

If you prefer explicit setup steps, use this order:

```bash
loopforge init
loopforge config validate
loopforge doctor
```

If you want LoopForge to run that sequence for you and optionally verify one starter task, use:

```bash
loopforge onboard --workspace loopforge-onboard-demo
```

For config field details, see [Config Reference](config.md).
For provider selection, see [Providers & Routing](../how-to/providers.md).

## `loopforge init` and `loopforge config validate`

Use these when you are still shaping configuration and do not want to run an agent yet.

```bash
loopforge init
loopforge config validate
loopforge config validate --json
```

Use `config validate` for syntax/schema issues.
Use `doctor` after that for runtime-readiness issues such as missing env vars, browser prerequisites, or provider connectivity.

## `loopforge onboard`

Recommended first command after install when you want a guided sanity check:

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

Run it anytime you get stuck or after changing providers:

```bash
loopforge doctor
loopforge doctor --json
loopforge doctor --strict
```

What `doctor` reports today:

- config/db paths
- config parsing
- router → provider mapping
- missing provider env vars
- security posture (`security.secrets`, `security.leaks`, `security.egress`)
- local Ollama connectivity (when configured)
- browser prerequisites
- required tooling such as Git

Text output ends with **Suggested next steps** when there is a likely remediation path.
JSON output keeps `summary` and `checks`, and also includes additive `next_actions` guidance.
`--strict` is useful in CI or preflight scripts because warnings become a non-zero exit.

## `loopforge agent run`

Use `agent run` for a one-shot session in a specific workspace:

```bash
loopforge agent run \
  --workspace loopforge-work \
  --prompt "Create hello.txt"
```

Important options:

- `--workspace` — required sandbox root
- `--prompt` — required user instruction
- `--kind <planning|coding|summary>` — selects router task kind
- `--session` — continue an earlier session id
- `--system` — inject an explicit system prompt
- `--allowed-tools` — apply a session-level tool allowlist

Use `agent run` when you want a direct task execution loop without harness lifecycle helpers.

## Long-running work with `harness`

Use harness mode when the work should keep durable workspace artifacts and be resumed incrementally:

```bash
loopforge harness init loopforge-task \
  --prompt "Initialize a refactor checklist"

loopforge harness run loopforge-task \
  --prompt "Continue with the next verified step"
```

Key difference from `agent run`:

- `agent run` is best for one focused session
- `harness init|run` is best for longer tasks that benefit from persistent bootstrap files, checkpoints, and repeatable continuation

## Skills, operations, and inspection

These command families are usually used after the core setup is already healthy:

- `loopforge skills ...` — inspect and execute local skills
- `loopforge channel drain` — send queued outbox messages once
- `loopforge channel worker` — keep draining the outbox in a long-lived process
- `loopforge daemon start` — expose the daemon HTTP API
- `loopforge acp events` / `checkpoints` — inspect event and delivery state
- `loopforge release check` — verify release metadata before tagging or publishing

## Examples

=== "macOS/Linux"
    ```bash
    loopforge init
    loopforge config validate
    loopforge doctor
    loopforge onboard --workspace loopforge-onboard-demo --starter workspace-brief

    mkdir -p loopforge-work
    loopforge agent run --workspace loopforge-work --prompt "Create hello.txt"

    loopforge harness init loopforge-task --prompt "Initialize a features checklist for refactoring this repo"
    loopforge harness run loopforge-task --prompt "Continue"
    ```

=== "Windows (PowerShell)"
    ```powershell
    loopforge init
    loopforge config validate
    loopforge doctor
    loopforge onboard --workspace loopforge-onboard-demo --starter workspace-brief

    mkdir loopforge-work
    loopforge agent run --workspace loopforge-work --prompt "Create hello.txt"

    loopforge harness init loopforge-task --prompt "Initialize a features checklist for refactoring this repo"
    loopforge harness run loopforge-task --prompt "Continue"
    ```
