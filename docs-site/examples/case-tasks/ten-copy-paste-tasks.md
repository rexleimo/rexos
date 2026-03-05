# 10 Copy/Paste Tasks (Beginner Friendly)

Use these as ready-to-run prompts.
Replace `my-work` with your own workspace folder.

## 0) One-time setup

```bash
ollama serve
loopforge init
mkdir -p my-work
```

## 1) Create a file

```bash
loopforge agent run --workspace my-work --prompt "Create notes/hello.md with exactly: Hello LoopForge"
```

## 2) Repo onboarding in 10 minutes

```bash
loopforge agent run --workspace . --prompt "You are helping me onboard this repo. Read top-level files, detect build/test commands, then write notes/onboarding.md."
```

## 3) Summarize Cargo dependencies

```bash
loopforge agent run --workspace . --prompt "Read Cargo.toml and write notes/deps.md with: dependency name, why it might exist, and risk notes."
```

## 4) Generate a test plan

```bash
loopforge agent run --workspace . --prompt "Inspect tests and write notes/test-plan.md with smoke, integration, and failure-injection cases."
```

## 5) Draft release notes

```bash
loopforge agent run --workspace . --prompt "Read recent commits and CHANGELOG.md, then write notes/release-draft.md for the next release."
```

## 6) Security quick scan memo

```bash
loopforge agent run --workspace . --prompt "Check config/docs for security-sensitive defaults and write notes/security-memo.md with findings and mitigations."
```

## 7) Web research memo with sources

```bash
loopforge agent run --workspace my-work --prompt "Research: local-first AI agent frameworks. Write notes/research.md with 5 bullets and source URLs."
```

## 8) Browser evidence capture

```bash
loopforge agent run --workspace my-work --prompt "Open https://www.wikipedia.org, extract 5 key facts, save screenshot to .rexos/browser/wiki.png, and write notes/wiki.md."
```

## 9) PDF summary

```bash
loopforge agent run --workspace my-work --prompt "Read samples/dummy.pdf with pdf tool and write notes/pdf-summary.md with key points and action items."
```

## 10) Refactor checklist before coding

```bash
loopforge agent run --workspace . --prompt "Analyze src/ and tests/, then write notes/refactor-checklist.md with safe sequencing and rollback plan."
```

## Verification checklist

After each task, verify artifacts exist:

```bash
find my-work -maxdepth 4 -type f | sort
```

For repository tasks:

```bash
find notes -maxdepth 2 -type f | sort
```

!!! tip
    If output quality is unstable, switch to a stronger model/provider for tool-heavy tasks.
