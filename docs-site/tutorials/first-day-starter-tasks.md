# First-Day Starter Tasks

If `loopforge onboard` succeeded, use one of these starter profiles next.

## Starter 1: `hello`

Best for the smallest sanity check.

```bash
loopforge onboard --workspace loopforge-onboard-demo --starter hello
```

Outcome:

- proves LoopForge can create files in the workspace
- keeps the first run extremely small

## Starter 2: `workspace-brief`

Best for converting setup into a useful artifact.

```bash
loopforge onboard --workspace loopforge-onboard-demo --starter workspace-brief
```

Outcome:

- writes `notes/workspace-brief.md`
- captures workspace purpose, risks, and next actions

## Starter 3: `repo-onboarding`

Best when you want LoopForge to start reading a real repository.

```bash
loopforge onboard --workspace loopforge-onboard-demo --starter repo-onboarding
```

Outcome:

- reads key repo files
- writes `notes/repo-onboarding.md`
- gives you a clearer first engineering handoff

## When to use `--prompt`

Use `--prompt` when you already know exactly what your first task should be:

```bash
loopforge onboard \
  --workspace loopforge-onboard-demo \
  --prompt "Read README.md and write notes/summary.md with 5 bullets and 3 next actions."
```

`--prompt` overrides starter defaults.
