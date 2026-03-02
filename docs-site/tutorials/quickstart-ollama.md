# Quickstart (Ollama)

This tutorial runs RexOS locally using Ollama’s OpenAI-compatible endpoint.

## Prerequisites

- Ollama is installed and running.
- You have at least one **chat model** available (example: `qwen3:4b`, `llama3.2`, etc.). (Embedding-only models won’t work.)

Check your local models:

```bash
ollama list
```

RexOS defaults to `providers.ollama.default_model = "llama3.2"` in `~/.rexos/config.toml`.

If you don’t have `llama3.2` installed, pick one of these:

1) Pull it:

```bash
ollama pull llama3.2
```

2) Or switch RexOS to a model you already have (example: `qwen3:4b`):

```toml
[providers.ollama]
default_model = "qwen3:4b"
```

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

=== "macOS/Linux"
    ```bash
    mkdir -p rexos-work
    rexos agent run --workspace rexos-work --prompt "Create hello.txt with the word hi"
    cat rexos-work/hello.txt
    ```

=== "Windows (PowerShell)"
    ```powershell
    mkdir rexos-work
    rexos agent run --workspace rexos-work --prompt "Create hello.txt with the word hi"
    Get-Content .\rexos-work\hello.txt
    ```

RexOS prints the final assistant output, and persists a stable `session_id` under `rexos-work/.rexos/session_id`.

## 4) Re-run in the same workspace (optional)

```bash
rexos agent run --workspace rexos-work --prompt "Now append a newline + bye to hello.txt"
```

## Next steps

- Use the harness for long tasks: see “Long Task With Harness”.
- Switch providers/models: see “Providers & Routing”.
