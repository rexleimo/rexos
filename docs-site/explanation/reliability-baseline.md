# Reliability Baseline

This page defines the minimum reliability signals LoopForge tracks during onboarding.

## Why this exists

The goal is simple:

- reduce time-to-first-success
- make onboarding failures diagnosable
- give both users and maintainers the same evidence trail

## Core onboarding artifacts

A successful or failed onboarding flow can produce three useful artifact types:

- `~/.loopforge/onboard-metrics.json`
- `~/.loopforge/onboard-events.jsonl`
- `<workspace>/.loopforge/onboard-report.json` and `.md`

Use them together:

- metrics show trends
- events show raw attempts
- the workspace report shows the last run's setup status, first-task status, and next actions

## Core metrics

LoopForge tracks:

- `attempted_first_task`
- `first_task_success`
- `first_task_failed`
- `failure_by_category`

Typical categories:

- `model_unavailable`
- `provider_unreachable`
- `tool_runtime_error`
- `sandbox_restriction`
- `unknown`

## Daily report script

LoopForge includes a reporting helper:

- `scripts/onboard_metrics_report.py`

Run it from repository root:

```bash
python3 scripts/onboard_metrics_report.py \
  --base-dir ~/.loopforge \
  --out-dir .tmp/onboard-report \
  --days 7 \
  --window-hours 24
```

It generates:

- `.tmp/onboard-report/onboard-report.json`
- `.tmp/onboard-report/onboard-report.md`

The Markdown report now includes:

- metrics snapshot
- recent failure categories
- **recommended fixes** for top failure types
- daily trend table

## Operational loop

1. Run `loopforge onboard`
2. Read `.loopforge/onboard-report.md` in the workspace
3. Run `loopforge doctor` if needed
4. Aggregate trends with `scripts/onboard_metrics_report.py`
5. Fix the top repeated failure category first
