# LoopForge

English | [简体中文](README.zh-CN.md)

LoopForge (formerly RexOS) is a long-running agent operating system: persistent memory, tool sandboxing, and model routing, plus an Anthropic-style harness for multi-session work.

## Brand update

- Public product name: **LoopForge**
- Primary CLI command: `loopforge`
- Compatibility names still in use: `rexos` (CLI alias), `~/.rexos` (config/data dir), and `rexleimo/rexos` (repo path)
- Existing scripts/docs using `rexos` continue to work

## Documentation

- Docs site: https://os.rexai.top
- (If the custom domain isn’t configured yet) GitHub Pages: https://rexleimo.github.io/rexos/

## Status

This repository is bootstrapped with a long-running harness (`features.json`, `init.sh`, `rexos-progress.md`). Work is tracked by flipping feature `passes` from `false` → `true`.

## Install

### Option A: Download a prebuilt binary (recommended)

Download the archive for your OS from GitHub Releases, extract it, and put `loopforge` (or `loopforge.exe`) somewhere on your `PATH`.
The compatibility command `rexos` is still included during migration.

### Option B: Build from source

```bash
# Install to ~/.cargo/bin (recommended for dev)
cargo install --path crates/rexos-cli --locked
loopforge --help

# Or build a local binary
cargo build --release -p rexos-cli
./target/release/loopforge --help
```

## Quick start (dev)

```bash
./init.sh
```

## Run with Ollama (OpenAI-compatible)

LoopForge defaults to `ollama` at `http://127.0.0.1:11434/v1` in `~/.rexos/config.toml`.

```bash
# 1) Start Ollama
ollama serve

# 2) Init LoopForge (compat command: rexos init)
loopforge init

# 3) Run an agent session in a workspace directory
mkdir -p /tmp/rexos-work
loopforge agent run --workspace /tmp/rexos-work --prompt "Create hello.txt with the word hi"
```

To run the optional Ollama smoke test: `REXOS_OLLAMA_MODEL=<your-model> cargo test -p rexos -- --ignored`.
To run the optional NVIDIA NIM smoke test: `NVIDIA_API_KEY=<key> cargo test -p rexos --test nvidia_nim_smoke -- --ignored`.

## Releasing (maintainers)

Pushing a `v*` tag triggers the Release workflow which attaches prebuilt archives to a GitHub Release.
Before every release, follow the versioning/changelog policy in `docs/versioning-and-release.md`.
If an iteration is marked as "needs version bump", the same change set must include both version number updates and changelog updates (`CHANGELOG.md`).

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Providers & routing

LoopForge supports multiple LLM providers via drivers:
- `openai_compatible` (Ollama / DeepSeek / Kimi / Qwen / GLM / MiniMax / NVIDIA NIM / OpenAI-compatible gateways)
- `dashscope_native` (Alibaba DashScope Generation API / Qwen native)
- `zhipu_native` (Zhipu GLM native auth/token handling)
- `minimax_native` (MiniMax native `text/chatcompletion_v2` API)
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
base_url = "https://api.deepseek.com"
api_key_env = "DEEPSEEK_API_KEY"
default_model = "deepseek-chat"

[router.coding]
provider = "ollama"
model = "default" # uses providers.<name>.default_model
```

To switch providers, set the provider's `api_key_env` (if needed) and update `[router.*]` to point at the provider you want. If you keep `model = "default"`, LoopForge uses `providers.<name>.default_model`.

Built-in presets include:
- `deepseek` (OpenAI-compatible)
- `kimi` / `kimi_cn` (OpenAI-compatible)
- `qwen` / `qwen_cn` / `qwen_sg` (OpenAI-compatible)
- `qwen_native` / `qwen_native_cn` / `qwen_native_sg` (DashScope native API)
- `glm` / `glm_native` (OpenAI-compatible / Zhipu native)
- `minimax` / `minimax_native` (OpenAI-compatible / MiniMax native)
- `nvidia` (OpenAI-compatible / NVIDIA NIM)
- `minimax_anthropic` (Anthropic-compatible gateway)
