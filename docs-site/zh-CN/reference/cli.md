# CLI 参考

LoopForge 的二进制命令是 `loopforge`。

## 我该用哪个命令？

如果你一时不确定从哪开始，可以按这个速查表：

- 首次安装或重置：`loopforge init`
- 校验配置语法与结构：`loopforge config validate`
- 排查环境是否可运行：`loopforge doctor`
- 想走一遍引导式首跑：`loopforge onboard`
- 在 workspace 里执行一次 agent 任务：`loopforge agent run`
- 需要可持续续跑的长任务：`loopforge harness init` + `loopforge harness run`
- 查看或执行本地 skills：`loopforge skills list|show|doctor|run`
- 发送积压的 outbox 消息：`loopforge channel drain` / `loopforge channel worker`
- 运行已存储的 cron（可选 worker）：`loopforge cron tick` / `loopforge cron worker`
- 以 HTTP 服务方式运行：`loopforge daemon start`
- 查看 ACP 事件与 checkpoint：`loopforge acp events` / `loopforge acp checkpoints`
- 发版前检查元数据：`loopforge release check`

## 命令族

顶层命令按使用场景分组如下：

- `loopforge init` —— 初始化 `~/.loopforge`（配置 + 数据库）
- `loopforge onboard` —— 一键 onboarding（`init` + 配置校验 + `doctor` + 可选首任务）
- `loopforge doctor` —— 诊断常见配置问题（配置文件、providers、浏览器、基础依赖）
- `loopforge config validate` —— 校验 `~/.loopforge/config.toml`
- `loopforge agent run` —— 在 workspace 中运行一次 agent session
- `loopforge harness init|run` —— 初始化并续跑长任务 harness workspace
- `loopforge skills list|show|doctor|run` —— 发现、查看、诊断并执行本地 skills
- `loopforge channel drain|worker` —— 发送 outbox 队列中的通知
- `loopforge cron tick|worker` —— 运行已存储的 cron（可选 worker）
- `loopforge daemon start` —— 启动 HTTP daemon
- `loopforge acp events|checkpoints` —— 查看 ACP 事件和投递 checkpoint
- `loopforge release check` —— 发版前检查 metadata 与预检项

## 推荐的首跑顺序

如果你想手动确认每一步，推荐按这个顺序执行：

```bash
loopforge init
loopforge config validate
loopforge doctor
```

如果你想让 LoopForge 帮你串起来，并且顺便验证一个 starter task，可以直接执行：

```bash
loopforge onboard --workspace loopforge-onboard-demo
```

配置字段说明见 [配置参考](config.md)。
Provider 选择策略见 [Providers 与路由](../how-to/providers.md)。

## `loopforge init` 与 `loopforge config validate`

当你还在调配置、不想立刻跑 agent 时，先用这两个命令：

```bash
loopforge init
loopforge config validate
loopforge config validate --json
```

`config validate` 适合看语法 / schema 问题。
`doctor` 更适合看运行期准备度问题，比如缺失环境变量、浏览器前置条件或 provider 连通性。

## `loopforge onboard`

安装后想快速验证一遍时，最推荐先跑：

```bash
loopforge onboard --workspace loopforge-onboard-demo
```

常用参数：

- `--skip-agent` —— 只做环境与配置检查，不跑首个 agent 任务
- `--starter <hello|workspace-brief|repo-onboarding>` —— 选择 starter task 档位
- `--prompt "..."` —— 用显式 prompt 覆盖 starter 默认任务
- `--timeout-ms <n>` —— 调整 doctor 探测超时

行为顺序：

1. 确保 `~/.loopforge` 已初始化
2. 校验配置
3. 运行 `loopforge doctor`
4. 可选地执行首个 agent 任务
5. 对内置 starter，只有真的生成了目标产物才会判定成功
6. 在 workspace 里生成 onboarding 报告：
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

卡住时，或者切换 provider 之后，先跑它：

```bash
loopforge doctor
loopforge doctor --json
loopforge doctor --strict
```

当前会检查：

- config/db 路径
- 配置解析
- router → provider 映射
- 缺失的 provider 环境变量
- 安全姿态（`security.secrets`、`security.leaks`、`security.egress`）
- 本地 Ollama 连通性（在已配置时）
- 浏览器前置条件
- Git 等基础工具

当存在明显修复路径时，文本输出末尾会追加 **Suggested next steps**。
JSON 保留 `summary` 和 `checks`，并额外提供 `next_actions` 建议。
`--strict` 适合 CI 或发版前预检，因为只要有 warning 就会非零退出。

## `loopforge agent run`

`agent run` 适合在指定 workspace 里跑一次 one-shot agent 任务：

```bash
loopforge agent run \
  --workspace loopforge-work \
  --prompt "Create hello.txt"
```

关键参数：

- `--workspace` —— 必填，工具沙盒根目录
- `--prompt` —— 必填，用户任务指令
- `--kind <planning|coding|summary>` —— 选择本次路由类型
- `--session` —— 复用既有 session id
- `--system` —— 传入额外 system prompt
- `--allowed-tools` —— 为本次 session 额外收紧工具白名单

如果你只想直接执行任务，而不需要 harness 的生命周期管理，就用 `agent run`。

## 用 `harness` 跑长任务

当任务需要持续续跑、保留 bootstrap 文件和中间产物时，使用 harness：

```bash
loopforge harness init loopforge-task \
  --prompt "Initialize a refactor checklist"

loopforge harness run loopforge-task \
  --prompt "Continue with the next verified step"
```

和 `agent run` 的区别：

- `agent run` 更适合一次性的聚焦执行
- `harness init|run` 更适合需要持久化工作目录、checkpoint 和续跑节奏的长任务

## Skills、运维与检查命令

这组命令通常在基础环境已经健康后再使用：

- `loopforge skills ...` —— 查看和执行本地 skills
- `loopforge channel drain` —— 单次发送 outbox 中待投递消息
- `loopforge channel worker` —— 长驻发送 outbox 消息
- `loopforge daemon start` —— 启动 daemon HTTP API
- `loopforge acp events` / `checkpoints` —— 查看事件和投递状态
- `loopforge release check` —— 打 tag / 发布前做元数据预检

## 示例

=== "macOS/Linux"
    ```bash
    loopforge init
    loopforge config validate
    loopforge doctor
    loopforge onboard --workspace loopforge-onboard-demo --starter workspace-brief

    mkdir -p loopforge-work
    loopforge agent run --workspace loopforge-work --prompt "Create hello.txt"

    loopforge harness init loopforge-task --prompt "Initialize a features checklist for refactoring this repo"
    loopforge harness run loopforge-task --prompt "Continue"
    ```

=== "Windows (PowerShell)"
    ```powershell
    loopforge init
    loopforge config validate
    loopforge doctor
    loopforge onboard --workspace loopforge-onboard-demo --starter workspace-brief

    mkdir loopforge-work
    loopforge agent run --workspace loopforge-work --prompt "Create hello.txt"

    loopforge harness init loopforge-task --prompt "Initialize a features checklist for refactoring this repo"
    loopforge harness run loopforge-task --prompt "Continue"
    ```
