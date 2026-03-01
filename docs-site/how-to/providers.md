# Providers & Routing

RexOS loads provider config from `~/.rexos/config.toml` and routes each task kind (planning/coding/summary) to a `(provider, model)` pair.

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
default_model = "qwen3:4b"

[router.coding]
provider = "ollama"
model = "default"
```

## Example: GLM (Zhipu native)

```toml
[providers.glm_native]
kind = "zhipu_native"
base_url = "https://open.bigmodel.cn/api/paas/v4"
api_key_env = "ZHIPU_API_KEY" # "id.secret"
default_model = "glm-4.5"

[router.coding]
provider = "glm_native"
model = "default"
```

## Example: MiniMax (native)

```toml
[providers.minimax_native]
kind = "minimax_native"
base_url = "https://api.minimax.chat"
api_key_env = "MINIMAX_API_KEY"
default_model = "abab6.5-chat"

[router.coding]
provider = "minimax_native"
model = "default"
```

## Routing tips

- Use `model = "default"` to pick `providers.<name>.default_model`.
- Keep **local dev** on `ollama`, and route “bigger” runs to a cloud provider by changing `[router.*]`.

