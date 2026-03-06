# LoopForge Onboarding Iteration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Upgrade LoopForge first-run UX by enhancing `onboard`, adding actionable doctor/report guidance, and reorganizing docs around a tighter install-to-success path.

**Architecture:** Extend the existing CLI entrypoints instead of introducing new top-level systems. Keep report and remediation logic additive so current JSON consumers stay compatible, then update docs and navigation so the new first-day flow is visible to users.

**Tech Stack:** Rust workspace (`clap`, existing LoopForge runtime), Python `unittest`, MkDocs.

---

### Task 1: Lock doctor remediation behavior with targeted tests

**Files:**
- Modify: `crates/loopforge-cli/src/doctor.rs`
- Test: `crates/loopforge-cli/src/doctor.rs`

**Step 1: Add failing tests for next-action generation**

Add tests that build a small `DoctorReport` or check list and assert:
- missing config/database yields an action mentioning `loopforge init`
- missing provider env vars yields an action mentioning the relevant env var setup
- missing browser prerequisites yields an action mentioning `LOOPFORGE_BROWSER_CHROME_PATH` or `LOOPFORGE_BROWSER_CDP_HTTP`
- text output includes `Suggested next steps`

**Step 2: Run targeted tests to confirm failure**

Run: `cargo test -p loopforge-cli doctor_ --locked`
Expected: FAIL because `next_actions`/rendering behavior does not exist yet.

**Step 3: Implement minimal additive report fields**

Implement in `crates/loopforge-cli/src/doctor.rs`:
- add `next_actions: Vec<String>` to `DoctorReport`
- derive the list from existing check ids/statuses
- keep existing `summary` and `checks` fields unchanged
- extend `to_text()` to append `Suggested next steps` only when non-empty

**Step 4: Re-run targeted tests**

Run: `cargo test -p loopforge-cli doctor_ --locked`
Expected: PASS.

### Task 2: Add starter profiles and onboarding report artifacts

**Files:**
- Modify: `crates/loopforge-cli/src/main.rs`
- Test: `crates/loopforge-cli/src/main.rs`

**Step 1: Add failing CLI parse and helper tests**

Add tests that assert:
- `loopforge onboard --workspace demo --starter hello` parses
- starter `workspace-brief` resolves to the expected prompt text
- onboarding report serialization writes both JSON and Markdown artifacts

**Step 2: Run targeted tests to confirm failure**

Run: `cargo test -p loopforge-cli cli_parses_onboard_subcommand --locked`
Run: `cargo test -p loopforge-cli onboard_ --locked`
Expected: FAIL because `--starter` and report helpers do not exist yet.

**Step 3: Implement minimal onboarding enhancements**

Implement in `crates/loopforge-cli/src/main.rs`:
- add `--starter <hello|workspace-brief|repo-onboarding>` to `Command::Onboard`
- keep `--prompt` as explicit override when provided
- generate:
  - `.loopforge/onboard-report.json`
  - `.loopforge/onboard-report.md`
  under the chosen workspace
- print a closing summary that includes:
  - config result
  - doctor counts
  - first-task outcome or skip status
  - recommended next command
  - starter suggestions

**Step 4: Re-run targeted tests**

Run: `cargo test -p loopforge-cli onboard_ --locked`
Expected: PASS.

### Task 3: Make onboarding metrics reports actionable

**Files:**
- Modify: `scripts/onboard_metrics_report.py`
- Test: `scripts/tests/test_onboard_metrics_report.py`

**Step 1: Add failing tests for failure recommendations**

Extend Python tests to assert that Markdown/JSON report output includes:
- top failure categories
- recommended remediation text for categories such as `model_unavailable` and `provider_unreachable`

**Step 2: Run script tests to confirm failure**

Run: `python3 -m unittest scripts.tests.test_onboard_metrics_report`
Expected: FAIL because the recommendations are not yet in the report.

**Step 3: Implement minimal recommendation mapping**

Update `scripts/onboard_metrics_report.py` to:
- map common failure categories to short fix suggestions
- include them in both generated Markdown and JSON
- keep existing report keys stable unless an additive extension is needed

**Step 4: Re-run script tests**

Run: `python3 -m unittest scripts.tests.test_onboard_metrics_report`
Expected: PASS.

### Task 4: Refresh the user docs path

**Files:**
- Modify: `README.md`
- Modify: `docs-site/index.md`
- Modify: `docs-site/reference/cli.md`
- Modify: `docs-site/zh-CN/reference/cli.md`
- Modify: `docs-site/tutorials/new-user-walkthrough.md`
- Modify: `docs-site/explanation/reliability-baseline.md`
- Modify: `docs-site/zh-CN/explanation/reliability-baseline.md`
- Create: `docs-site/tutorials/first-day-starter-tasks.md`
- Create: `docs-site/zh-CN/tutorials/first-day-starter-tasks.md`
- Create: `docs-site/how-to/onboarding-troubleshooting.md`
- Create: `docs-site/zh-CN/how-to/onboarding-troubleshooting.md`
- Modify: `mkdocs.yml`

**Step 1: Draft the new linear journey**

Update docs so the main path is:
- install
- `loopforge onboard`
- first successful task
- starter tasks
- troubleshooting

**Step 2: Reflect real CLI behavior only**

Document:
- `--starter`
- workspace report artifacts
- `doctor` suggested next actions
- how to recover from common onboarding failures

**Step 3: Add navigation entries**

Add new pages to `mkdocs.yml` so users can discover them from the main nav.

**Step 4: Run docs build**

Run: `python3 -m mkdocs build --strict`
Expected: PASS.

### Task 5: Publish the competitor-follow-up narrative

**Files:**
- Create: `docs-site/blog/loopforge-next-iteration-openfang-openclaw.md`
- Modify: `docs-site/blog/index.md`
- Modify: `mkdocs.yml`

**Step 1: Write the blog post**

Cover:
- what OpenFang does well in getting-started/templates
- what OpenClaw does well in help/testing/troubleshooting
- why LoopForge is absorbing first-run UX improvements now instead of copying the larger surface area

**Step 2: Surface the post**

Add the new post to the blog index and nav.

**Step 3: Re-run docs build**

Run: `python3 -m mkdocs build --strict`
Expected: PASS.

### Task 6: Run full focused verification

**Files:**
- No new files

**Step 1: Run Rust CLI tests**

Run: `cargo test -p loopforge-cli --locked`
Expected: PASS.

**Step 2: Run Python report tests**

Run: `python3 -m unittest scripts.tests.test_onboard_metrics_report`
Expected: PASS.

**Step 3: Run docs build**

Run: `python3 -m mkdocs build --strict`
Expected: PASS.

**Step 4: Run local onboarding smoke without live model dependency**

Run: `cargo run -p loopforge-cli -- onboard --workspace .tmp/onboard-demo --skip-agent`
Expected:
- PASS
- `.tmp/onboard-demo/.loopforge/onboard-report.json` exists
- `.tmp/onboard-demo/.loopforge/onboard-report.md` exists

**Step 5: Optional live Ollama smoke**

Run: `cargo run -p loopforge-cli -- onboard --workspace .tmp/onboard-demo --starter workspace-brief`
Expected: PASS when Ollama is running and a chat model is configured.
