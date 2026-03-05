# Reliability Baseline

This page defines the minimum reliability signals LoopForge tracks for new-user onboarding.

## Why this baseline exists

The goal is simple: reduce time-to-first-success and make onboarding failures diagnosable.

## Core onboarding metrics

LoopForge tracks first-task onboarding outcomes in `~/.rexos/onboard-metrics.json`:

- `attempted_first_task`: how many onboarding runs actually attempted the first agent task
- `first_task_success`: successful first-task runs
- `first_task_failed`: failed first-task runs
- `failure_by_category`: grouped failures (for example `model_unavailable`, `provider_unreachable`)

From these values:

- **First-task success rate** = `first_task_success / attempted_first_task`

## Failure event log

Each onboarding result is also appended to:

- `~/.rexos/onboard-events.jsonl`

Each line records timestamp, workspace, session id, outcome, and (for failures) category + error summary.

## Read current metrics

=== "macOS/Linux"
    ```bash
    cat ~/.rexos/onboard-metrics.json
    tail -n 20 ~/.rexos/onboard-events.jsonl
    ```

=== "Windows (PowerShell)"
    ```powershell
    Get-Content $HOME/.rexos/onboard-metrics.json
    Get-Content $HOME/.rexos/onboard-events.jsonl -Tail 20
    ```

## Suggested initial targets

- First-task success rate >= 70%
- `model_unavailable` and `provider_unreachable` combined < 50% of failures
- Median time-to-first-success <= 3 minutes (track externally if needed)

## Operational loop

1. Run onboarding
2. Check failure categories
3. Fix top category first (model setup / provider connectivity / config)
4. Re-run onboarding and compare trend
