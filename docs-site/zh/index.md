# RexOS

简体中文 | [English](../index.md)

RexOS 是一个面向长任务的 Agent OS：**持久化记忆**、**工具沙盒**、**模型路由**，以及用于跨多次会话推进任务的 Anthropic 风格 **Harness**。

## 你可以用 RexOS 做什么

- 让 agent 在多次运行之间持续工作（SQLite 记录 session/history）。
- 在 workspace 目录内安全执行工具调用（读写文件 / shell / web_fetch）。
- 针对不同任务类型（planning/coding/summary）路由到不同 provider/model。
- 用 harness 做“可持续推进”的长任务：`features.json` checklist + `init.sh`/`init.ps1` 验证 + git checkpoint。

## 快速开始（本地 Ollama）

```bash
ollama serve
rexos init

mkdir -p /tmp/rexos-work
rexos agent run --workspace /tmp/rexos-work --prompt "Create hello.txt with the word hi"
```

下一步：看左侧教程，尤其是 Harness 长任务。

