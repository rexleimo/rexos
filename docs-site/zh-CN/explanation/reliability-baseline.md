# 可靠性基线

本页定义 LoopForge 在新用户 onboarding 阶段最小化追踪的可靠性信号。

## 为什么需要这套基线

目标很直接：降低“首次成功时间”，并让 onboarding 失败可诊断、可归因。

## 核心 onboarding 指标

LoopForge 会把首任务结果写入 `~/.rexos/onboard-metrics.json`：

- `attempted_first_task`：实际尝试首个 agent 任务的次数
- `first_task_success`：首任务成功次数
- `first_task_failed`：首任务失败次数
- `failure_by_category`：按类别聚合失败（例如 `model_unavailable`、`provider_unreachable`）

可以计算：

- **首任务成功率** = `first_task_success / attempted_first_task`

## 失败事件日志

每次 onboarding 结果还会追加到：

- `~/.rexos/onboard-events.jsonl`

每行包含时间戳、workspace、session id、结果状态，以及失败时的分类与错误摘要。

## 查看当前指标

=== "macOS/Linux"
    ```bash
    cat ~/.rexos/onboard-metrics.json
    tail -n 20 ~/.rexos/onboard-events.jsonl
    ```

=== "Windows (PowerShell)"
    ```powershell
    Get-Content $HOME/.rexos/onboard-metrics.json
    Get-Content $HOME/.rexos/onboard-events.jsonl -Tail 20
    ```

## 日报汇总脚本

LoopForge 内置了一个 onboarding 指标日报脚本：

- `scripts/onboard_metrics_report.py`

在仓库根目录执行：

=== "macOS/Linux"
    ```bash
    python3 scripts/onboard_metrics_report.py \
      --base-dir ~/.rexos \
      --out-dir .tmp/onboard-report \
      --days 7 \
      --window-hours 24

    cat .tmp/onboard-report/onboard-report.md
    ```

=== "Windows (PowerShell)"
    ```powershell
    python scripts/onboard_metrics_report.py `
      --base-dir $HOME/.rexos `
      --out-dir .tmp/onboard-report `
      --days 7 `
      --window-hours 24

    Get-Content .tmp/onboard-report/onboard-report.md
    ```

输出文件：

- `.tmp/onboard-report/onboard-report.json`
- `.tmp/onboard-report/onboard-report.md`

## 初始目标建议

- 首任务成功率 >= 70%
- `model_unavailable` + `provider_unreachable` 之和占失败比重 < 50%
- 首次成功中位时间 <= 3 分钟（可通过外部埋点补充）

## 运维闭环

1. 跑 onboarding
2. 看失败分类
3. 先修复占比最高的分类（模型配置 / provider 连通性 / 配置错误）
4. 复跑并比较趋势
