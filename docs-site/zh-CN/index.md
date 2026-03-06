<div class="rexos-hero" markdown>

# LoopForge

**你的个人研发助理（Personal AI Engineer）。**

先用一条命令跑通第一条链路，再从 prompt 走到真实产物与可复现轨迹。

[onboard 一键上手](tutorials/new-user-walkthrough.md){ .md-button .md-button--primary }
[Starter Tasks](tutorials/first-day-starter-tasks.md){ .md-button }
[上手排障](how-to/onboarding-troubleshooting.md){ .md-button }
[为什么是 LoopForge](explanation/why-loopforge.md){ .md-button }

<p class="rexos-muted">
OpenClaw 更像个人生活助理。LoopForge 的定位是工程交付型 AI 助理：本地优先、产物导向、可审计。
</p>

</div>

> 产品名是 **LoopForge**。CLI 命令是 `loopforge`，运行目录仍是 `~/.loopforge`。

## 从这里开始

=== "1) 一键 onboarding"
    ```bash
    ollama serve
    loopforge onboard --workspace loopforge-onboard-demo
    ```

=== "2) 只检查环境，不跑首任务"
    ```bash
    loopforge onboard --workspace loopforge-onboard-demo --skip-agent
    ```

=== "3) 使用 starter profile"
    ```bash
    loopforge onboard --workspace loopforge-onboard-demo --starter workspace-brief
    ```

执行后会生成：

- `loopforge-onboard-demo/.loopforge/onboard-report.json`
- `loopforge-onboard-demo/.loopforge/onboard-report.md`

这两个文件会告诉你：哪里成功、哪里失败、下一步该怎么做。

<div class="grid cards" markdown>

- :material-rocket-launch: **Starter Tasks**
  不必自己从零写 prompt，直接选高价值首任务。
  [打开 starter tasks](tutorials/first-day-starter-tasks.md)

- :material-stethoscope: **上手排障**
  快速定位首轮常见问题：配置、provider、浏览器、模型可用性。
  [打开排障页](how-to/onboarding-troubleshooting.md)

- :material-hammer-wrench: **修一个失败测试**
  让 LoopForge 跑测试、修一个失败用例，并输出 `notes/fix-report.md`。
  [可复制任务](examples/case-tasks/fix-one-failing-test.md)

- :material-history: **可复现推进**
  固化流程：修改 -> 验证 -> checkpoint。
  [Harness 工作流](tutorials/harness-long-task.md)

</div>

## 我们的定位

- 如果你最关心工程交付、可复现执行、真实产物，选 **LoopForge**。
- 如果你更关心生活助理式对话体验，选助手类产品。
- 如果你更关心多渠道覆盖，选平台型产品。

## 下一步

- [新人 walkthrough](tutorials/new-user-walkthrough.md)
- [Starter Tasks](tutorials/first-day-starter-tasks.md)
- [上手排障](how-to/onboarding-troubleshooting.md)
- [5 分钟可见结果](tutorials/five-minute-outcomes.md)
- [案例任务库](examples/case-tasks/index.md)
