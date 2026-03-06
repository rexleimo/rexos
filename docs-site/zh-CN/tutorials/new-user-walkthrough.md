# 新人 walkthrough（10 分钟）

这是验证 LoopForge 在你机器上能否跑通的最快路径。

## 0) 前置条件

- `loopforge` 已安装并加入 `PATH`
- Ollama 已启动：`ollama serve`
- 你至少有一个可用的对话模型：

```bash
ollama list
```

如果 `llama3.2` 没有安装，可以先拉取，或者把 `~/.loopforge/config.toml` 改成你本机已有的模型。

## 1) 推荐第一跑：`onboard`

```bash
loopforge onboard --workspace loopforge-onboard-demo
```

预期：

- 配置校验通过
- doctor 输出摘要
- 首个任务运行一次
- LoopForge 打印推荐的下一条命令
- 生成以下报告产物：
  - `loopforge-onboard-demo/.loopforge/onboard-report.json`
  - `loopforge-onboard-demo/.loopforge/onboard-report.md`

如果你只想先检查环境：

```bash
loopforge onboard --workspace loopforge-onboard-demo --skip-agent
```

如果你想跑一个比 `hello.txt` 更有价值的首任务：

```bash
loopforge onboard --workspace loopforge-onboard-demo --starter workspace-brief
```

## 2) 打开 onboarding 报告

打开：

- `loopforge-onboard-demo/.loopforge/onboard-report.md`

这份报告会告诉你：

- 哪些步骤成功
- 哪些步骤失败
- 推荐下一步做什么
- 下一批 starter tasks 是什么

## 3) 在同一 workspace 里继续一次任务

如果 onboarding 成功，再跑一个具体任务：

```bash
loopforge agent run --workspace loopforge-onboard-demo --prompt "Continue from the current workspace and write notes/next-steps.md with 3 follow-up actions."
```

## 4) 第一天下一步做什么

你可以继续看：

- [Starter Tasks](first-day-starter-tasks.md)
- [5 分钟可见结果](five-minute-outcomes.md)
- [案例任务](../examples/case-tasks/index.md)
- [上手排障](../how-to/onboarding-troubleshooting.md)

## 5) 如果失败了

运行：

```bash
loopforge doctor
```

再结合以下三处建议排查：

- 终端里的 Suggested next steps
- `loopforge-onboard-demo/.loopforge/onboard-report.md`
- [上手排障](../how-to/onboarding-troubleshooting.md)
