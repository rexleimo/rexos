# RexOS

RexOS is a long-running agent operating system: persistent memory, tool sandboxing, and model routing, plus an Anthropic-style harness for multi-session work.

## Status

This repository is bootstrapped with a long-running harness (`features.json`, `init.sh`, `rexos-progress.md`). Work is tracked by flipping feature `passes` from `false` → `true`.

## Quick start (dev)

```bash
./init.sh
```

## Run with Ollama (OpenAI-compatible)

RexOS defaults to `ollama` at `http://127.0.0.1:11434/v1` in `~/.rexos/config.toml`.

```bash
# 1) Start Ollama
ollama serve

# 2) Init RexOS (creates ~/.rexos/config.toml + ~/.rexos/rexos.db)
cargo run -- init

# 3) Run an agent session in a workspace directory
mkdir -p /tmp/rexos-work
cargo run -- agent run --workspace /tmp/rexos-work --prompt "Create hello.txt with the word hi"
```

To run the optional Ollama smoke test: `REXOS_OLLAMA_MODEL=<your-model> cargo test -- --ignored`.

## Providers & routing

RexOS supports multiple LLM providers via drivers:
- `openai_compatible` (Ollama / DeepSeek / Kimi / Qwen / GLM / MiniMax / OpenAI-compatible gateways)
- `anthropic` (Claude API + Anthropic-compatible gateways)
- `gemini` (Google Gemini API)

`~/.rexos/config.toml` defines providers and routes each task kind to a `(provider, model)` pair:

```toml
[providers.ollama]
kind = "openai_compatible"
base_url = "http://127.0.0.1:11434/v1"
api_key_env = ""
default_model = "llama3.2"

[providers.deepseek]
kind = "openai_compatible"
base_url = "https://api.deepseek.com/v1"
api_key_env = "DEEPSEEK_API_KEY"
default_model = "deepseek-chat"

[router.coding]
provider = "ollama"
model = "llama3.2"
```

To switch providers, set the provider's `api_key_env` (if needed) and update `[router.*]` to point at the provider + model you want.
