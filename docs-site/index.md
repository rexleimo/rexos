# RexOS

RexOS is a long-running agent operating system: **persistent memory**, **sandboxed tools**, and **model routing**, plus an Anthropic-style **harness** for multi-session work.

## What you can do with RexOS

- Keep an agent working across many runs (SQLite-backed session history + a durable workspace).
- Run tools safely in a workspace sandbox (`fs_*`, `shell`, `web_fetch`).
- Route different task types (planning/coding/summary) to different providers/models.
- Use the harness to iterate on a checklist (`features.json`) with repeatable verification (`init.sh` / `init.ps1`) and git checkpoints.

## Quickstart (local, with Ollama)

1) Install `rexos` (from GitHub Releases, or build from source).

2) Start Ollama:

```bash
ollama serve
```

3) Initialize RexOS (creates `~/.rexos/config.toml` + `~/.rexos/rexos.db`):

```bash
rexos init
```

4) Run a session in a workspace directory:

```bash
mkdir -p /tmp/rexos-work
rexos agent run --workspace /tmp/rexos-work --prompt "Create hello.txt with the word hi"
```

Next: read the tutorials for Harness-based long tasks.

## Popular use cases

- “Keep improving this repo until the test suite passes” (harness + checkpoints).
- “Apply the same change across many files safely” (workspace sandbox + git).
- “Build a release pipeline and verify it” (tools + incremental commits).
- “Try provider-native APIs (GLM/MiniMax/Qwen) but keep local dev on Ollama.”

## Links

- Tutorials: see the left navigation.
- Repo: use the “Edit on GitHub” button to contribute fixes quickly.

