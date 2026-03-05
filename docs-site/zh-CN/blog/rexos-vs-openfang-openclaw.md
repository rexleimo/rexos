# LoopForge 与 OpenFang/OpenClaw 对比（开发者视角）

本文基于当前工作区在 **2026 年 3 月 4 日** 的本地快照做对比，目标是帮开发团队更快选型。

## 一句话结论

- 如果你最关注“编码任务可复现 + 可追踪交付”，优先 **LoopForge**（原 RexOS）。
- 如果你最关注“多渠道运营 + 大量适配器”，优先 **OpenFang**。
- 如果你最关注“个人助手体验 + 多端多渠道覆盖”，优先 **OpenClaw**。

## 三者各自最强项

| 项目 | 最适合场景 | 核心原因 |
|---|---|---|
| LoopForge | 工程研发流程、长任务推进 | Harness 循环（修改->验证->checkpoint）、workspace 沙盒、SQLite 记忆、CLI 优先 |
| OpenFang | 多 Agent 运营和渠道分发 | 渠道覆盖与模板体系很重，偏运营执行面 |
| OpenClaw | 个人助手产品形态 | 文档和渠道覆盖极广，上手向导完整 |

## 对研发团队最关键的差异

### 1）可复现交付链路

LoopForge 的核心优势是工程化循环：

```bash
loopforge harness init my-repo
loopforge harness run my-repo --prompt "Run tests and fix one failing case"
```

如果你的团队以“可复现 checkpoint + 产物留痕”衡量进展，这种模式更稳。

### 2）本地起步成本

LoopForge 默认就是本地 Ollama 开发流：

```bash
ollama serve
loopforge init
loopforge agent run --workspace rexos-work --prompt "Create hello.txt with the word hi"
```

对工程团队来说，首跑成本更低。

### 3）文档策略

从竞品学习到的要点：
- OpenFang：安装矩阵清晰，FAQ/故障排查结构化。
- OpenClaw：上手漏斗完整，场景文档数量巨大。

LoopForge 对应优化（本次已开始补齐）：
- 新手 FAQ
- 更多可复制案例任务
- 面向增长的对比博客

## 快速选型规则

1. 首要目标是“研发稳定交付”？-> **LoopForge**
2. 首要目标是“运营渠道铺开”？-> **OpenFang**
3. 首要目标是“个人助手体验覆盖”？-> **OpenClaw**

## 3 条命令先跑 LoopForge

```bash
ollama serve
loopforge init
loopforge agent run --workspace rexos-demo --prompt "Create notes/plan.md with a 7-day migration checklist"
```

## 继续阅读

- [新手 FAQ](../how-to/faq.md)
- [新人复习](../tutorials/new-user-walkthrough.md)
- [10 个可复制任务](../examples/case-tasks/ten-copy-paste-tasks.md)
