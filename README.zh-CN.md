# LoopForge

[English](README.md) | 简体中文

LoopForge（原 RexOS）是一个长任务的 Agent OS：持久化记忆、工具沙盒、模型路由，以及一个 Anthropic 风格的 Harness，用于跨多次会话持续推进任务。

## 品牌更新

- 对外产品名：**LoopForge**
- 主 CLI 命令：`loopforge`
- 兼容名仍保留：`rexos`（CLI 兼容别名）、`~/.rexos`（配置/数据目录）、`rexleimo/rexos`（仓库路径）
- 现有脚本和文档中的 `rexos` 命令可继续使用

## 文档

- 文档站点：https://os.rexai.top
- （如自定义域名未配置）GitHub Pages：https://rexleimo.github.io/rexos/

## 状态

本仓库已用长任务 harness 引导初始化（`features.json`、`init.sh`、`rexos-progress.md`）。推进方式是把每个 feature 的 `passes` 从 `false` 置为 `true`，并保持 checklist 稳定。

## 安装

### 方案 A：下载预编译二进制（推荐）

从 GitHub Releases 下载对应你系统的压缩包，解压后把 `loopforge`（或 `loopforge.exe`）放到 `PATH` 里即可。
迁移期间仍包含 `rexos` 兼容命令。

### 方案 B：从源码构建

```bash
# 安装到 ~/.cargo/bin（开发推荐）
cargo install --path crates/rexos-cli --locked
loopforge --help

# 或仅构建本地二进制
cargo build --release -p rexos-cli
./target/release/loopforge --help
```

## 快速开始（开发）

```bash
./init.sh
```

## 使用 Ollama（OpenAI 兼容）

LoopForge 默认配置会在 `~/.rexos/config.toml` 里把 `ollama` 指向 `http://127.0.0.1:11434/v1`。

```bash
# 1) 启动 Ollama
ollama serve

# 2) 初始化 LoopForge（兼容命令：rexos init）
loopforge init

# 3) 在某个 workspace 目录里运行一次 agent session
mkdir -p /tmp/rexos-work
loopforge agent run --workspace /tmp/rexos-work --prompt "Create hello.txt with the word hi"
```

可选的 Ollama smoke test：`REXOS_OLLAMA_MODEL=<your-model> cargo test -p rexos -- --ignored`。
可选的 NVIDIA NIM smoke test：`NVIDIA_API_KEY=<key> cargo test -p rexos --test nvidia_nim_smoke -- --ignored`。

## 发版（维护者）

推送一个 `v*` tag 会触发 Release 工作流，构建并把预编译压缩包上传到 GitHub Release。
每次发版前请遵循 `docs/versioning-and-release.md` 的版本与更新说明规则。
如果本次迭代被标记为“需要升级版本号”，则同一批改动必须同时包含版本号更新和 `CHANGELOG.md` 更新。

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Providers 与路由

LoopForge 通过多种 driver 支持多个 LLM Provider：
- `openai_compatible`（Ollama / DeepSeek / Kimi / Qwen / GLM / MiniMax / NVIDIA NIM / 其它 OpenAI-compatible 网关）
- `dashscope_native`（阿里云 DashScope Generation API / Qwen 原生）
- `zhipu_native`（智谱 GLM 原生：auth/token 处理）
- `minimax_native`（MiniMax 原生 `text/chatcompletion_v2` API）
- `anthropic`（Claude API + Anthropic-compatible 网关）
- `gemini`（Google Gemini API）

在 `~/.rexos/config.toml` 中配置 providers，并把不同任务类型路由到 `(provider, model)`：

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

切换 provider：配置对应 provider 的 `api_key_env`（如需），并把 `[router.*]` 指向你想用的 provider；如果 `model = "default"`，LoopForge 会使用 `providers.<name>.default_model`。

内置 presets 包含：
- `deepseek`（OpenAI-compatible）
- `kimi` / `kimi_cn`（OpenAI-compatible）
- `qwen` / `qwen_cn` / `qwen_sg`（OpenAI-compatible）
- `qwen_native` / `qwen_native_cn` / `qwen_native_sg`（DashScope 原生 API）
- `glm` / `glm_native`（OpenAI-compatible / 智谱原生）
- `minimax` / `minimax_native`（OpenAI-compatible / MiniMax 原生）
- `nvidia`（OpenAI-compatible / NVIDIA NIM）
- `minimax_anthropic`（Anthropic-compatible 网关）
