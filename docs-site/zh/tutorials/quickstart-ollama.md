# 快速开始（Ollama）

本教程用 Ollama 的 OpenAI 兼容接口在本地跑通 RexOS。

## 1) 启动 Ollama

```bash
ollama serve
```

## 2) 初始化 RexOS

会创建：
- `~/.rexos/config.toml`
- `~/.rexos/rexos.db`

```bash
rexos init
```

## 3) 运行第一次 session

工具调用会被沙盒限制在 `--workspace` 目录内：

```bash
mkdir -p /tmp/rexos-work
rexos agent run --workspace /tmp/rexos-work --prompt "Create hello.txt with the word hi"
```

## 下一步

- Harness 长任务：见 “Harness 长任务”
- Providers 与路由：见 “Providers 与路由”

