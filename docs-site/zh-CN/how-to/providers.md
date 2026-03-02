# Providers 与路由

RexOS 从 `~/.rexos/config.toml` 读取 providers 配置，并把每个任务类型（planning/coding/summary）路由到 `(provider, model)`。

## 开箱即用的 presets

执行 `rexos init` 后，`~/.rexos/config.toml` 默认已经包含常用 providers（可直接改路由使用）：

- 本地：`ollama`
- OpenAI-compatible：`deepseek`、`kimi` / `kimi_cn`、`qwen` / `qwen_cn` / `qwen_sg`、`glm`、`minimax`、`nvidia`
- Provider-native：`qwen_native*`、`glm_native`、`minimax_native`
- 网关：`minimax_anthropic`
- 官方 API：`anthropic`、`gemini`

你通常只需要做两件事：

1) 配好对应的 API key 环境变量（如果需要）
2) 把 `[router.*]` 指向你想用的 provider

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
default_model = "llama3.2"

[router.coding]
provider = "ollama"
model = "default"
```

## 示例：GLM（智谱原生）

```toml
[providers.glm_native]
kind = "zhipu_native"
base_url = "https://open.bigmodel.cn/api/paas/v4"
api_key_env = "ZHIPUAI_API_KEY" # 通常是 "id.secret"
default_model = "glm-4"

[router.coding]
provider = "glm_native"
model = "default"
```

!!! tip "智谱 key 格式"
    如果 `ZHIPUAI_API_KEY` 形如 `id.secret`，RexOS 会自动签发短期 JWT（无需你手动生成 token）。

## 示例：MiniMax（原生）

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

## 示例：NVIDIA NIM（OpenAI 兼容）

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

## 路由建议

- 研发调试：优先用 `ollama`（小模型先跑通逻辑）
- 线上/更强能力：把 `[router.*]` 切到 GLM/MiniMax/Qwen 等云端 provider

常见组合：planning 用本地小模型，coding 用云端强模型：

```toml
[router.planning]
provider = "ollama"
model = "default"

[router.coding]
provider = "glm_native" # 或 minimax_native / deepseek / kimi / qwen_native ...
model = "default"

[router.summary]
provider = "ollama"
model = "default"
```

## API keys（环境变量）

RexOS 会从 `api_key_env` 指定的环境变量读取 key。

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

## 可选 smoke tests（真实 provider）

这些测试会真实请求 provider endpoint，并且默认标记为 `#[ignore]`：

```bash
# Ollama（OpenAI-compatible）
REXOS_OLLAMA_MODEL=<your-model> cargo test -p rexos --test ollama_smoke -- --ignored

# GLM（智谱原生）
ZHIPUAI_API_KEY=<id.secret> REXOS_GLM_MODEL=<model> cargo test -p rexos --test zhipu_smoke -- --ignored

# MiniMax（原生）
MINIMAX_API_KEY=<key> REXOS_MINIMAX_MODEL=<model> cargo test -p rexos --test minimax_smoke -- --ignored

# NVIDIA NIM（OpenAI-compatible）
NVIDIA_API_KEY=<key> REXOS_NVIDIA_MODEL=<model> cargo test -p rexos --test nvidia_nim_smoke -- --ignored
```
