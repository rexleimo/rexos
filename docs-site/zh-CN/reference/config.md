# 配置参考（`~/.rexos/config.toml`）

RexOS 把配置存放在 `~/.rexos/config.toml`。

## Providers

每个 provider 条目包含：

- `kind`：驱动类型（`openai_compatible`、`zhipu_native`、`minimax_native` 等）
- `base_url`：API base URL
- `api_key_env`：读取 API key 的环境变量名（本地 provider 可为空）
- `default_model`：当路由里写 `model = "default"` 时使用的默认模型名

示例：

```toml
[providers.ollama]
kind = "openai_compatible"
base_url = "http://127.0.0.1:11434/v1"
api_key_env = ""
default_model = "llama3.2"
```

## Router

每个任务类型会选择一个 `(provider, model)`：

```toml
[router.planning]
provider = "ollama"
model = "default"

[router.coding]
provider = "ollama"
model = "default"

[router.summary]
provider = "ollama"
model = "default"
```

## 内置 presets

RexOS 默认包含一些常用 provider presets（名称可能会演进）：

- OpenAI-compatible：`deepseek`、`kimi`、`qwen`、`glm`、`minimax`
- Provider-native：`glm_native`、`minimax_native`、`qwen_native`

