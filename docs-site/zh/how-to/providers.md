# Providers 与路由

RexOS 从 `~/.rexos/config.toml` 读取 providers 配置，并把每个任务类型（planning/coding/summary）路由到 `(provider, model)`。

## Provider kinds

- `openai_compatible`：OpenAI 兼容 Chat Completions（Ollama / DeepSeek / Kimi / …）
- `zhipu_native`：智谱 GLM 原生（内置 JWT 处理）
- `minimax_native`：MiniMax 原生 `text/chatcompletion_v2`
- `dashscope_native`：阿里云 DashScope 原生
- `anthropic`：Claude API
- `gemini`：Google Gemini API

## 示例：本地 Ollama

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

## 路由建议

- 研发调试：优先用 `ollama`（小模型先跑通逻辑）
- 线上/更强能力：把 `[router.*]` 切到 GLM/MiniMax/Qwen 等云端 provider

