# LoopForge vs OpenFang/OpenClaw (Builder View)

This comparison is based on local snapshots in this workspace on **March 4, 2026**.
Goal: help builders choose the right tool faster.

## TL;DR

- Choose **LoopForge** (formerly RexOS) if your core job is coding workflows with reproducible checkpoints and local-first model routing.
- Choose **OpenFang** if your core job is multi-channel operations and autonomous "hands" at scale.
- Choose **OpenClaw** if your core job is personal assistant experiences across many channels and devices.

## What each project optimizes for

| Project | Strongest fit | Why |
|---|---|---|
| LoopForge | Dev workflows + long-running engineering tasks | Harness loop (`change -> verify -> checkpoint`), workspace sandbox, SQLite memory, CLI-first |
| OpenFang | Multi-agent operations + channel adapters | Heavy emphasis on channels, templates, operational breadth |
| OpenClaw | Personal assistant platform + device/channel surfaces | Massive docs and channel coverage, onboarding wizard, broad ecosystem integration |

## Practical differences for builders

### 1) Reproducibility loop

LoopForge pushes a strict engineering loop:

```bash
loopforge harness init my-repo
loopforge harness run my-repo --prompt "Run tests and fix one failing case"
```

If your team measures progress by repeatable checkpoints and artifact trails, this pattern is very effective.

### 2) Local-first bring-up

LoopForge default path is optimized for local bring-up with Ollama:

```bash
ollama serve
loopforge init
loopforge agent run --workspace rexos-work --prompt "Create hello.txt with the word hi"
```

This keeps first-run friction low for engineering teams.

### 3) Documentation style

What we learned from competitors:
- OpenFang: clear install matrix + strong troubleshooting/FAQ structure.
- OpenClaw: strong onboarding funnel + huge scenario inventory + broad links.

What LoopForge should do (and now starts doing):
- add beginner FAQ
- add more copy/paste task packs
- add growth blog positioning pages

## Decision rule

Use this simple decision tree:

1. Need coding workflow reliability first? -> **LoopForge**
2. Need broad channel operations as first priority? -> **OpenFang**
3. Need consumer-like personal assistant breadth? -> **OpenClaw**

## Start with LoopForge in 3 commands

```bash
ollama serve
loopforge init
loopforge agent run --workspace rexos-demo --prompt "Create notes/plan.md with a 7-day migration checklist"
```

## Next reading

- [Beginner FAQ](../how-to/faq.md)
- [New User Walkthrough](../tutorials/new-user-walkthrough.md)
- [10 Copy/Paste Tasks](../examples/case-tasks/ten-copy-paste-tasks.md)
