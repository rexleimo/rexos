# LoopForge Onboarding + Reliability + Growth Iteration Design

## Context

This design covers the next medium-sized LoopForge iteration in `meos/`.
The scope is intentionally limited to LoopForge itself. `openfang/` and `.tmp/openclaw/` are reference inputs only and are not modified.

## Goal

Improve the first-day LoopForge experience across four tracks at once:

1. **Product capability**: make `loopforge onboard` feel like a complete first-run entrypoint, not just a setup wrapper.
2. **Stability**: turn `doctor` and onboarding outputs into clearer remediation guidance.
3. **Docs growth**: create a tighter new-user path from install to first useful task.
4. **Competitor follow-up**: convert the strongest onboarding/help patterns from OpenFang and OpenClaw into LoopForge-native, smaller-scope improvements.

## What This Design Is For

### User-visible changes

These are changes users can directly see in the CLI or docs:

- `loopforge onboard` prints a clearer “what passed / what failed / what next” summary.
- `loopforge onboard` supports a small set of first-day starter task profiles.
- onboarding writes a readable report artifact into the workspace.
- README and docs site route users through a more obvious install → onboard → starter tasks → troubleshooting journey.
- a new public blog post explains the iteration and what LoopForge is borrowing from competitor strengths.

### Internal / developer-facing changes

These are implementation details that support the experience:

- `doctor` grows structured next-action hints while keeping current JSON compatibility.
- onboarding report generation is formalized so tests and docs can rely on it.
- onboarding metrics reporting gets more actionable failure guidance.
- design and implementation are documented in `docs/plans/` for future sessions.

## Non-Goals

This iteration does **not** do the following:

- no full template marketplace
- no web dashboard or control UI
- no multi-channel expansion modeled after OpenClaw
- no direct code changes in `openfang/` or `.tmp/openclaw/`
- no large runtime architecture rewrite

## Research Summary

### Current LoopForge strengths

LoopForge already has useful building blocks:

- `loopforge onboard` exists and already composes `init + config validate + doctor + optional first task`
- `loopforge doctor` already checks config, providers, browser prerequisites, and basic tooling
- onboarding metrics already persist to `~/.loopforge/onboard-metrics.json` and `~/.loopforge/onboard-events.jsonl`
- docs already contain quickstart, new-user walkthrough, FAQ, reliability baseline, blog, and case-task content

The main gap is not absence of features. The gap is **first-run clarity and packaging**.

### What OpenFang does well

Observed strengths from `openfang/`:

- deeper getting-started flow
- more explicit template/catalog framing
- stronger operator/maintainer checklist mindset

What LoopForge should borrow now:

- make “what can I do first?” more obvious
- present starter tasks as guided outcomes
- keep a clear path from quickstart to repeatable workflows

### What OpenClaw does well

Observed strengths from `.tmp/openclaw/`:

- stronger help/testing/troubleshooting documentation coverage
- more obvious operational docs for common breakpoints
- clearer trust/help signals for complex environments

What LoopForge should borrow now:

- stronger troubleshooting entrypoints
- more explicit suggested next actions after checks fail
- better docs sequencing around first-run failure recovery

## Recommended Approach

Use a **focused composition** approach instead of a big new system.

- Extend existing `onboard` rather than creating a separate wizard.
- Extend existing `doctor` rather than inventing another diagnostics command.
- Package first-day workflows as starter tasks/docs rather than building a full templates subsystem.
- Use docs and report artifacts to make the upgrade feel larger than the code churn actually is.

This keeps the iteration small enough to verify while producing clear user-visible value.

## Detailed Design

### 1. Onboard becomes a guided first-run entrypoint

`loopforge onboard` remains the recommended first command after install.

Additions:

- support a starter selector such as:
  - `hello`
  - `workspace-brief`
  - `repo-onboarding`
- keep `--prompt` as the override when users want full control
- generate workspace-local onboarding report artifacts:
  - `.loopforge/onboard-report.json`
  - `.loopforge/onboard-report.md`
- print a consistent closing summary with:
  - config result
  - doctor summary
  - first-task outcome
  - suggested next command
  - starter-task suggestions

Why this matters:

- beginners get an obvious next action
- maintainers get a durable artifact to inspect when onboarding fails
- docs can point at concrete generated files instead of vague expectations

### 2. Doctor adds remediation hints without breaking compatibility

`loopforge doctor --json` should remain backward compatible for existing workflows that depend on `summary` and `checks`.

Additions:

- include `next_actions` as an extra JSON field
- derive next actions from common states such as:
  - missing config/database
  - router points to unknown provider
  - missing provider API keys
  - local Ollama unreachable
  - browser prerequisites missing
  - required tools missing
- append a `Suggested next steps` section in text mode

Why this matters:

- current output tells users what is wrong
- the new output tells them what to do next

### 3. Onboarding metrics report becomes more actionable

The Python report script already aggregates outcomes.
This iteration adds guidance, not a new reporting system.

Additions:

- report top failure categories with recommended fixes
- link the failures back to the new troubleshooting documentation structure
- keep output as JSON + Markdown

Why this matters:

- support loops become shorter
- docs and automation can share the same language for common failures

### 4. Docs are reorganized around the first-day path

The docs should become more linear for new users.

Recommended path:

1. Install / Quickstart
2. `loopforge onboard`
3. First successful task
4. Starter tasks
5. Troubleshooting
6. Deeper references

New or expanded pages:

- starter tasks page with 3–5 copy/paste prompts
- onboarding troubleshooting page
- refreshed CLI reference for `onboard` and `doctor`
- refreshed new-user walkthrough
- homepage and README copy that prioritize `onboard`

### 5. Competitor follow-up is turned into public product positioning

Add a blog post that explains:

- which usability and trust cues were strong in OpenFang and OpenClaw
- which of those LoopForge chose to absorb now
- why LoopForge is intentionally not copying the larger surface area yet

This helps product positioning and creates a visible narrative for iteration quality.

## File-Level Design

### User-visible docs and content

- `README.md`
- `docs-site/index.md`
- `docs-site/reference/cli.md`
- `docs-site/zh-CN/reference/cli.md`
- `docs-site/tutorials/new-user-walkthrough.md`
- `docs-site/tutorials/first-day-starter-tasks.md`
- `docs-site/zh-CN/tutorials/first-day-starter-tasks.md`
- `docs-site/how-to/onboarding-troubleshooting.md`
- `docs-site/zh-CN/how-to/onboarding-troubleshooting.md`
- `docs-site/explanation/reliability-baseline.md`
- `docs-site/zh-CN/explanation/reliability-baseline.md`
- `docs-site/blog/loopforge-next-iteration-openfang-openclaw.md`
- `mkdocs.yml`

### Internal implementation and tests

- `crates/loopforge-cli/src/main.rs`
- `crates/loopforge-cli/src/doctor.rs`
- `scripts/onboard_metrics_report.py`
- `scripts/tests/test_onboard_metrics_report.py`

## Validation Strategy

### Required validation

- `cargo test -p loopforge-cli`
- `python3 -m unittest scripts.tests.test_onboard_metrics_report`
- `python3 -m mkdocs build --strict`
- `cargo run -p loopforge-cli -- onboard --workspace .tmp/onboard-demo --skip-agent`

### Optional live validation

When a real Ollama instance is available:

- `cargo run -p loopforge-cli -- onboard --workspace .tmp/onboard-demo --starter workspace-brief`

## Risks

- adding too much branching logic into `main.rs` could make the CLI harder to maintain
- onboarding copy can become noisy if too much information is printed at once
- documentation changes can drift from actual CLI behavior if report artifact formats are not locked by tests

## Mitigations

- keep the starter system intentionally tiny
- keep `doctor` JSON additive, not breaking
- lock the new text/report behavior with focused unit tests
- treat docs and CLI output as one package and verify them together

## Success Criteria

### User-visible

- a beginner can run `loopforge onboard` and immediately see the recommended next action
- starter tasks are discoverable from README, homepage, and tutorial flow
- failure recovery has a clearly linked troubleshooting path

### Internal

- `doctor` output stays backward compatible for existing JSON consumers
- onboarding report artifacts are generated deterministically
- test coverage exists for new CLI/report behavior
- docs build passes in strict mode
