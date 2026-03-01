# Quickstart (Ollama)

This tutorial runs RexOS locally using Ollama’s OpenAI-compatible endpoint.

## Prerequisites

- Ollama is installed and running.
- You have at least one chat model available (example: `qwen3:4b`, `llama3.2`, etc.).

## 1) Start Ollama

```bash
ollama serve
```

## 2) Initialize RexOS

This creates:
- `~/.rexos/config.toml` (provider config + routing)
- `~/.rexos/rexos.db` (SQLite memory)

```bash
rexos init
```

## 3) Run your first agent session

Pick a workspace directory (tools are sandboxed to this root):

```bash
mkdir -p /tmp/rexos-work
rexos agent run --workspace /tmp/rexos-work --prompt "Create hello.txt with the word hi"
```

RexOS prints the final assistant output, and also logs a `session_id` to stderr for later reuse.

## 4) Re-run with the same session id (optional)

```bash
rexos agent run --workspace /tmp/rexos-work --session <SESSION_ID> --prompt "Now append a newline + bye to hello.txt"
```

## Next steps

- Use the harness for long tasks: see “Long Task With Harness”.
- Switch providers/models: see “Providers & Routing”.

