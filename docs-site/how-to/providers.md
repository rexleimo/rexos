# Providers & Routing

RexOS loads provider config from `~/.rexos/config.toml` and routes each task kind (planning/coding/summary) to a `(provider, model)` pair.

## Built-in presets (out of the box)

After `rexos init`, your `~/.rexos/config.toml` already includes common providers and sensible defaults:

- Local: `ollama`
- OpenAI-compatible: `deepseek`, `kimi` / `kimi_cn`, `qwen` / `qwen_cn` / `qwen_sg`, `glm`, `minimax`, `nvidia`
- Provider-native: `qwen_native*`, `glm_native`, `minimax_native`
- Gateways: `minimax_anthropic`
- First-party APIs: `anthropic`, `gemini`

You typically only need to:

1) set the corresponding API key env var (if any)
2) point `[router.*]` at the provider you want

## Provider kinds

- `openai_compatible`: OpenAI-compatible Chat Completions APIs (Ollama, DeepSeek, Kimi, many gateways)
- `dashscope_native`: Alibaba DashScope native API (Qwen native)
- `zhipu_native`: Zhipu GLM native API (JWT auth handled)
- `minimax_native`: MiniMax native `text/chatcompletion_v2`
- `anthropic`: Claude API (and compatible gateways)
- `gemini`: Google Gemini API

## Example: Ollama (local)

```toml
[providers.ollama]
kind = "openai_compatible"
base_url = "http://127.0.0.1:11434/v1"
api_key_env = ""
default_model = "llama3.2"

[router.coding]
provider = "ollama"
model = "default"
```

## Example: GLM (Zhipu native)

```toml
[providers.glm_native]
kind = "zhipu_native"
base_url = "https://open.bigmodel.cn/api/paas/v4"
api_key_env = "ZHIPUAI_API_KEY" # typically "id.secret"
default_model = "glm-4"

[router.coding]
provider = "glm_native"
model = "default"
```

!!! tip "Zhipu auth format"
    If `ZHIPUAI_API_KEY` looks like `id.secret`, RexOS will sign a short-lived JWT automatically.

## Example: MiniMax (native)

```toml
[providers.minimax_native]
kind = "minimax_native"
base_url = "https://api.minimax.chat/v1"
api_key_env = "MINIMAX_API_KEY"
default_model = "MiniMax-M2.5"

[router.coding]
provider = "minimax_native"
model = "default"
```

## Example: NVIDIA NIM (OpenAI-compatible)

```toml
[providers.nvidia]
kind = "openai_compatible"
base_url = "https://integrate.api.nvidia.com/v1"
api_key_env = "NVIDIA_API_KEY"
default_model = "meta/llama-3.2-3b-instruct"

[router.coding]
provider = "nvidia"
model = "default"
```

## Routing tips

- Use `model = "default"` to pick `providers.<name>.default_model`.
- Prefer “local planning” + “cloud coding” for cost/perf:

```toml
[router.planning]
provider = "ollama"
model = "default"

[router.coding]
provider = "glm_native" # or minimax_native / deepseek / kimi / qwen_native ...
model = "default"

[router.summary]
provider = "ollama"
model = "default"
```

## API keys (env vars)

RexOS reads provider keys from the env var referenced by `api_key_env`.

## Optional smoke tests (real providers)

These tests hit real provider endpoints and are `#[ignore]` by default:

```bash
# Ollama (OpenAI-compatible)
REXOS_OLLAMA_MODEL=<your-model> cargo test -p rexos --test ollama_smoke -- --ignored

# GLM (Zhipu native)
ZHIPUAI_API_KEY=<id.secret> REXOS_GLM_MODEL=<model> cargo test -p rexos --test zhipu_smoke -- --ignored

# MiniMax (native)
MINIMAX_API_KEY=<key> REXOS_MINIMAX_MODEL=<model> cargo test -p rexos --test minimax_smoke -- --ignored

# NVIDIA NIM (OpenAI-compatible)
NVIDIA_API_KEY=<key> REXOS_NVIDIA_MODEL=<model> cargo test -p rexos --test nvidia_nim_smoke -- --ignored
```

=== "Bash (macOS/Linux)"
    ```bash
    export DEEPSEEK_API_KEY="..."
    export MOONSHOT_API_KEY="..."
    export DASHSCOPE_API_KEY="..."
    export ZHIPUAI_API_KEY="id.secret"
    export MINIMAX_API_KEY="..."
    export NVIDIA_API_KEY="..."
    ```

=== "PowerShell (Windows)"
    ```powershell
    $env:DEEPSEEK_API_KEY = "..."
    $env:MOONSHOT_API_KEY = "..."
    $env:DASHSCOPE_API_KEY = "..."
    $env:ZHIPUAI_API_KEY = "id.secret"
    $env:MINIMAX_API_KEY = "..."
    $env:NVIDIA_API_KEY = "..."
    ```
