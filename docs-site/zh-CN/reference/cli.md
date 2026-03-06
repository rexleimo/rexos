# CLI 参考

LoopForge 的二进制命令是 `loopforge`。

## 顶层命令

- `loopforge init` — 初始化 `~/.loopforge`（配置 + 数据库）
- `loopforge onboard` — 一键 onboarding（`init` + 配置校验 + `doctor` + 可选首任务）
- `loopforge doctor` — 诊断常见配置问题（配置文件、providers、浏览器、基础依赖）
- `loopforge agent run` — 在 workspace 中运行一次 agent session
- `loopforge skills list|show|doctor|run` — 发现、查看、诊断并执行本地 skills
- `loopforge harness init` — 初始化 harness workspace（持久化产物 + git）
- `loopforge harness run` — 运行一次增量 harness session
- `loopforge channel drain` / `worker` — 发送 outbox 队列中的通知
- `loopforge daemon start` — 启动 HTTP daemon

## `loopforge onboard`

安装后的推荐第一条命令：

```bash
loopforge onboard --workspace loopforge-onboard-demo
```

常用参数：

- `--skip-agent` — 只做环境与配置检查，不跑首个 agent 任务
- `--starter <hello|workspace-brief|repo-onboarding>` — 选择 starter task 档位
- `--prompt "..."` — 用显式 prompt 覆盖 starter 默认任务
- `--timeout-ms <n>` — 调整 doctor 探测超时

行为顺序：

1. 确保 `~/.loopforge` 已初始化
2. 校验配置
3. 运行 `loopforge doctor`
4. 可选地执行首个 agent 任务
5. 在 workspace 里生成 onboarding 报告：
   - `.loopforge/onboard-report.json`
   - `.loopforge/onboard-report.md`

报告会包含：

- 配置状态
- doctor 摘要
- 建议下一步
- 首任务状态
- 推荐下一条命令
- starter suggestions

## `loopforge doctor`

卡住时先跑：

```bash
loopforge doctor
```

机器可读输出：

```bash
loopforge doctor --json
```

当前会检查：

- config/db 路径
- 配置解析
- router → provider 映射
- 缺失的 provider 环境变量
- 本地 Ollama 连通性（在已配置时）
- 浏览器前置条件
- Git 等基础工具

当存在明显修复路径时，文本输出末尾会追加 **Suggested next steps**。
JSON 保留 `summary` 和 `checks`，并额外提供 `next_actions` 建议。
