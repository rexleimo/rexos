<div class="rexos-hero" markdown>

# LoopForge

**Your Personal AI Engineer for real shipping work.**

Start with one command, get a real artifact, and keep a reproducible trail from prompt to deliverable.

[Start with onboard](tutorials/new-user-walkthrough.md){ .md-button .md-button--primary }
[Starter tasks](tutorials/first-day-starter-tasks.md){ .md-button }
[Troubleshooting](how-to/onboarding-troubleshooting.md){ .md-button }
[Why LoopForge](explanation/why-loopforge.md){ .md-button }

<p class="rexos-muted">
OpenClaw is closer to a personal assistant. LoopForge is optimized for builders: local-first, artifact-oriented, and audit-friendly.
</p>

</div>

> Product name: **LoopForge**. CLI: `loopforge`. Runtime data path: `~/.loopforge`.

## Start Here

=== "1) One-command onboarding"
    ```bash
    ollama serve
    loopforge onboard --workspace loopforge-onboard-demo
    ```

=== "2) Skip agent, verify setup only"
    ```bash
    loopforge onboard --workspace loopforge-onboard-demo --skip-agent
    ```

=== "3) Use a starter profile"
    ```bash
    loopforge onboard --workspace loopforge-onboard-demo --starter workspace-brief
    ```

After `onboard`, LoopForge writes:

- `loopforge-onboard-demo/.loopforge/onboard-report.json`
- `loopforge-onboard-demo/.loopforge/onboard-report.md`

These files tell you what passed, what failed, and what to do next.

<div class="grid cards" markdown>

- :material-rocket-launch: **First-Day Starter Tasks**
  Pick a guided first task instead of writing a prompt from scratch.
  [Open starter tasks](tutorials/first-day-starter-tasks.md)

- :material-stethoscope: **Onboarding Troubleshooting**
  Fix common first-run issues quickly: config, provider, browser, or model setup.
  [Open troubleshooting](how-to/onboarding-troubleshooting.md)

- :material-hammer-wrench: **Fix One Failing Test**
  Ask LoopForge to run tests, repair one failure, and write `notes/fix-report.md`.
  [Copy/paste prompt](examples/case-tasks/fix-one-failing-test.md)

- :material-history: **Reproducible Progress**
  Keep a clear trail: change -> verify -> checkpoint.
  [Harness workflow](tutorials/harness-long-task.md)

</div>

## Where We Fit

- Choose **LoopForge** when you care most about engineering delivery, reproducibility, and useful artifacts.
- Choose assistant-style products when you care most about everyday personal chat workflows.
- Choose broad operations platforms when you need channel coverage first.

## Next Steps

- [New user walkthrough](tutorials/new-user-walkthrough.md)
- [Starter tasks](tutorials/first-day-starter-tasks.md)
- [Onboarding troubleshooting](how-to/onboarding-troubleshooting.md)
- [5-minute outcomes](tutorials/five-minute-outcomes.md)
- [Case task library](examples/case-tasks/index.md)
