# 新人复习（10 分钟）

这个教程是一个“安装后自检流程”，帮你确认 RexOS 的核心链路都跑通：

- 本地模型（Ollama）可用
- 工具调用被沙盒限制在 workspace 内
- 同一个 `session_id` 可以多次续跑（记忆持久化）
- harness workspace 会产出持久化文件 + git checkpoint

## 0) 前置条件

- `rexos` 已安装并在 `PATH` 中
- Ollama 正在运行：`ollama serve`
- Ollama 里至少有一个 **对话模型**：

```bash
ollama list
```

如果默认模型（`llama3.2`）没有拉取，你可以：

```bash
ollama pull llama3.2
```

或者编辑 `~/.rexos/config.toml`，把默认模型切到你已有的模型，例如：

```toml
[providers.ollama]
default_model = "qwen3:4b" # 示例：换成你本机已有的模型
```

## 1) 初始化 RexOS

```bash
rexos init
```

预期产物：

- `~/.rexos/config.toml`
- `~/.rexos/rexos.db`

## 2) 跑一次 one-shot session（workspace 沙盒）

=== "macOS/Linux"
    ```bash
    mkdir -p rexos-demo
    rexos agent run --workspace rexos-demo --prompt "Create hello.txt with the word hi"
    cat rexos-demo/hello.txt
    ```

=== "Windows (PowerShell)"
    ```powershell
    mkdir rexos-demo
    rexos agent run --workspace rexos-demo --prompt "Create hello.txt with the word hi"
    Get-Content .\rexos-demo\hello.txt
    ```

预期：

- workspace 里生成 `hello.txt`，内容为 `hi`
- stderr 会打印 `session_id`，并且会持久化到 `rexos-demo/.rexos/session_id`

## 3) 在同一个 workspace 里续跑（记忆）

```bash
rexos agent run --workspace rexos-demo --prompt "Append a newline + bye to hello.txt"
```

验证文件已更新：

=== "macOS/Linux"
    ```bash
    cat rexos-demo/hello.txt
    ```

=== "Windows (PowerShell)"
    ```powershell
    Get-Content .\rexos-demo\hello.txt
    ```

## 4) 创建 harness workspace（持久化产物 + git）

=== "macOS/Linux"
    ```bash
    mkdir -p rexos-harness-demo
    rexos harness init rexos-harness-demo
    ```

=== "Windows (PowerShell)"
    ```powershell
    mkdir rexos-harness-demo
    rexos harness init rexos-harness-demo
    ```

在 `rexos-harness-demo/` 里你应该能看到：

- `features.json`
- `rexos-progress.md`
- `init.sh` 和 `init.ps1`
- `.git/`（且已经有一条初始化 commit）

运行一次 preflight（不带 prompt）：

```bash
rexos harness run rexos-harness-demo
```

## 5) 文档按钮（可复现性）

文档站点的每个页面都应该有：

- **编辑此页** → 跳转到 GitHub 的 `docs-site/...`
- **查看源文件** → 打开 raw Markdown

如果按钮不见了或不可用，检查 docs workflow 以及 `mkdocs.yml`（`repo_url` + `edit_uri`）。
