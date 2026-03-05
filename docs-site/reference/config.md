# Config Reference (`~/.rexos/config.toml`)

LoopForge stores configuration in `~/.rexos/config.toml` (path kept for compatibility).

## Providers

Each provider entry defines:

- `kind`: driver kind (`openai_compatible`, `zhipu_native`, `minimax_native`, etc.)
- `base_url`: API base URL
- `api_key_env`: environment variable name that contains the API key (empty for local providers)
- `default_model`: default model for `model = "default"`

Example:

```toml
[providers.ollama]
kind = "openai_compatible"
base_url = "http://127.0.0.1:11434/v1"
api_key_env = ""
default_model = "qwen3:4b"
```

## Router

Each task kind selects a `(provider, model)` pair:

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

## Built-in presets

LoopForge includes common provider presets (names may evolve):

- OpenAI-compatible: `deepseek`, `kimi`, `qwen`, `glm`, `minimax`
- Provider-native: `glm_native`, `minimax_native`, `qwen_native`
