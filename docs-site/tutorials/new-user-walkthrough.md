# New User Walkthrough (10 minutes)

This is the fastest way to prove LoopForge is usable on your machine.

## 0) Prerequisites

- `loopforge` is installed and on your `PATH`
- Ollama is running: `ollama serve`
- you have at least one chat model available:

```bash
ollama list
```

If `llama3.2` is not installed, either pull it or set a model you already have in `~/.loopforge/config.toml`.

## 1) Recommended first run: `onboard`

```bash
loopforge onboard --workspace loopforge-onboard-demo
```

Expected:

- config validation passes
- doctor output prints a summary
- the first task runs once
- LoopForge prints a recommended next command
- these report artifacts exist:
  - `loopforge-onboard-demo/.loopforge/onboard-report.json`
  - `loopforge-onboard-demo/.loopforge/onboard-report.md`

If you only want to validate setup first:

```bash
loopforge onboard --workspace loopforge-onboard-demo --skip-agent
```

If you want a more useful first task than `hello.txt`:

```bash
loopforge onboard --workspace loopforge-onboard-demo --starter workspace-brief
```

## 2) Read the onboarding report

Open:

- `loopforge-onboard-demo/.loopforge/onboard-report.md`

This report tells you:

- what passed
- what failed
- what LoopForge recommends next
- which starter tasks you can run next

## 3) Continue with one more task

If onboarding succeeded, run one more concrete task in the same workspace:

```bash
loopforge agent run --workspace loopforge-onboard-demo --prompt "Continue from the current workspace and write notes/next-steps.md with 3 follow-up actions."
```

## 4) Pick a first-day direction

Choose one of these next:

- [Starter tasks](first-day-starter-tasks.md)
- [5-minute outcomes](five-minute-outcomes.md)
- [Case tasks](../examples/case-tasks/index.md)
- [Onboarding troubleshooting](../how-to/onboarding-troubleshooting.md)

## 5) If something failed

Run:

```bash
loopforge doctor
```

Then use the suggested next steps in:

- terminal output
- `loopforge-onboard-demo/.loopforge/onboard-report.md`
- [Onboarding troubleshooting](../how-to/onboarding-troubleshooting.md)
