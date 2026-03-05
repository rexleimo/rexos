# 什么是 LoopForge？（原 RexOS）

LoopForge 是 LoopForge 的对外新品牌名。
这次改名的目标是：更好记、更易传播、更利于在开发者社区建立心智。

## 一句话定位

LoopForge 是一个面向工程交付的 **local-first 长任务 Agent OS**，
强调 `修改 -> 验证 -> checkpoint` 的可复现循环。

## 适合谁

- 希望获得“可复现编码闭环”，而不只是一次性聊天输出的开发者。
- 需要稳定 checkpoint、产物留痕、workspace 安全执行工具的团队。
- 先在本地 Ollama 跑通，再按需路由到更强云模型的构建者。

## 改了什么，没改什么

已变化：
- 产品/对外品牌名：**LoopForge**
- CLI 命令：`loopforge`

保持连续性：
- 配置与数据目录：`~/.rexos`

## 为什么叫 LoopForge

- `Loop`：代表长任务的迭代循环
- `Forge`：代表锻造可交付的软件产物
- 合在一起：既表达过程，也表达结果

## 3 条命令首跑

```bash
ollama serve
loopforge init
loopforge agent run --workspace loopforge-demo --prompt "Create notes/hello.md with a short intro to LoopForge."
```

## 品牌关键词（文档/搜索/社区）

- LoopForge
- 长任务 Agent OS
- 本地优先 coding agent
- harness 工作流
- 可复现 AI 工程循环

## 继续阅读

- [LoopForge 与 OpenFang/OpenClaw 对比（开发者视角）](rexos-vs-openfang-openclaw.md)
- [新人复习](../tutorials/new-user-walkthrough.md)
- [10 个可复制任务](../examples/case-tasks/ten-copy-paste-tasks.md)
