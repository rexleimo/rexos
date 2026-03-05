# What Is LoopForge? (Formerly LoopForge)

LoopForge is the new public brand of the product previously known as LoopForge.
We renamed to make the product easier to remember and easier to spread in builder communities.

## One-line positioning

LoopForge is a **local-first long-running Agent OS** for engineering workflows:
`change -> verify -> checkpoint`.

## Who it is for

- Developers who want reproducible coding loops, not one-shot chat outputs.
- Teams that need durable checkpoints, artifact trails, and workspace-safe tool execution.
- Builders who start local with Ollama and later route to stronger cloud models.

## What changed vs what stayed the same

Changed:
- Product/public name: **LoopForge**
- CLI command: `loopforge`

Kept for continuity:
- Config and data dir: `~/.rexos`

## Why the name "LoopForge"

- `Loop`: long-running iterative workflows
- `Forge`: build, shape, and harden software artifacts
- Together: a toolchain identity that explains both process and outcome

## 3-command first run

```bash
ollama serve
loopforge init
loopforge agent run --workspace loopforge-demo --prompt "Create notes/hello.md with a short intro to LoopForge."
```

## Brand keywords (for docs/search/community)

- LoopForge
- long-running agent OS
- local-first coding agent
- harness workflow
- reproducible AI engineering loop

## Next reading

- [LoopForge vs OpenFang/OpenClaw (Builder View)](rexos-vs-openfang-openclaw.md)
- [New User Walkthrough](../tutorials/new-user-walkthrough.md)
- [10 Copy/Paste Tasks](../examples/case-tasks/ten-copy-paste-tasks.md)
