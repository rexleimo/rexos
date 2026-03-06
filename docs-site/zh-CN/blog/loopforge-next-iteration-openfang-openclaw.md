# LoopForge 下一轮从 OpenFang 和 OpenClaw 借鉴什么

## TL;DR

这一轮不会照搬 OpenFang 或 OpenClaw 的完整能力面。
LoopForge 只吸收对“首次成功”最关键的部分：

- 更清晰的 onboarding
- 更直接的 troubleshooting
- 更容易上手的 first-day tasks

## OpenFang 做得好的地方

OpenFang 很擅长把能力包装成“我第一天就能拿来干嘛”的体验。
Template/catalog 的表达方式，能让用户更快走到结果。

LoopForge 这轮吸收的是：

- starter-task 的表达
- 更紧凑的 getting-started 路径
- setup 完成后的下一步建议

## OpenClaw 做得好的地方

OpenClaw 在 help、testing、troubleshooting 和 operator trust signals 上很强。
很多首次失败并不是功能缺失，而是不知道哪里坏了、下一步该做什么。

LoopForge 这轮吸收的是：

- 更强的故障排查入口
- 更可执行的 doctor 建议
- 用 onboarding report 总结最近一次运行

## 为什么不是全部照搬

LoopForge 的核心任务不同：

- 工程交付
- 可复现执行
- 文件产物与 checkpoint
- 本地优先工作流

所以这轮只专注于“从安装到第一份工程产物”的最短路径。

## 这轮具体变化

- `loopforge onboard` 更像一个真正的 first-run 入口
- onboarding 会在 workspace 里写出 `.loopforge/onboard-report.json` 和 `.md`
- `loopforge doctor` 会给出更明确的 Suggested next steps
- 文档会更直接地引导用户经过 onboarding、starter tasks 和 troubleshooting
