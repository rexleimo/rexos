# First-Day Starter Tasks

如果 `loopforge onboard` 已经成功，可以继续用这些 starter profiles。

## Starter 1: `hello`

适合做最小自检。

```bash
loopforge onboard --workspace loopforge-onboard-demo --starter hello
```

效果：

- 证明 LoopForge 能在 workspace 里创建文件
- 首轮任务足够小，最容易跑通

## Starter 2: `workspace-brief`

适合把 setup 直接变成有价值的产物。

```bash
loopforge onboard --workspace loopforge-onboard-demo --starter workspace-brief
```

效果：

- 生成 `notes/workspace-brief.md`
- 梳理 workspace 用途、风险和下一步动作

## Starter 3: `repo-onboarding`

适合让 LoopForge 开始理解一个真实仓库。

```bash
loopforge onboard --workspace loopforge-onboard-demo --starter repo-onboarding
```

效果：

- 读取仓库关键文件
- 生成 `notes/repo-onboarding.md`
- 给后续工程任务一个更清晰的 handoff

## 什么时候用 `--prompt`

当你已经明确知道首个任务要做什么时，用 `--prompt`：

```bash
loopforge onboard \
  --workspace loopforge-onboard-demo \
  --prompt "Read README.md and write notes/summary.md with 5 bullets and 3 next actions."
```

`--prompt` 会覆盖 starter 默认任务。
