# 可靠性基线

本页定义 LoopForge 在 onboarding 阶段最小需要追踪的可靠性信号。

## 为什么需要这套基线

目标很直接：

- 降低首次成功时间
- 让 onboarding 失败可诊断
- 让用户和维护者看到同一套证据链

## 核心产物

一次成功或失败的 onboarding，通常会留下三类产物：

- `~/.loopforge/onboard-metrics.json`
- `~/.loopforge/onboard-events.jsonl`
- `<workspace>/.loopforge/onboard-report.json` 和 `.md`

组合使用方式：

- metrics 看趋势
- events 看原始尝试记录
- workspace 报告看最近一次运行的状态与下一步建议

## 核心指标

LoopForge 会追踪：

- `attempted_first_task`
- `first_task_success`
- `first_task_failed`
- `failure_by_category`

常见分类：

- `model_unavailable`
- `provider_unreachable`
- `tool_runtime_error`
- `sandbox_restriction`
- `unknown`

## 日报脚本

LoopForge 内置了：

- `scripts/onboard_metrics_report.py`

在仓库根目录运行：

```bash
python3 scripts/onboard_metrics_report.py \
  --base-dir ~/.loopforge \
  --out-dir .tmp/onboard-report \
  --days 7 \
  --window-hours 24
```

会生成：

- `.tmp/onboard-report/onboard-report.json`
- `.tmp/onboard-report/onboard-report.md`

Markdown 报告现在会包含：

- 指标快照
- 最近失败分类
- **top failure 的推荐修复动作**
- 每日趋势表

## 运维闭环

1. 运行 `loopforge onboard`
2. 打开 workspace 里的 `.loopforge/onboard-report.md`
3. 必要时运行 `loopforge doctor`
4. 用 `scripts/onboard_metrics_report.py` 看趋势
5. 优先修复重复出现最多的失败类型
