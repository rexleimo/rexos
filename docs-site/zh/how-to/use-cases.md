# 常见场景

## 1) 长任务“持续推进且可回滚”

用 harness，把循环固化成：修改 → 跑 init script → git checkpoint。

```bash
rexos harness init /tmp/task --prompt "把这个项目逐步改到测试全部通过"
rexos harness run /tmp/task --prompt "继续"
```

## 2) 多文件机械化改动

用 `agent run` + workspace 沙盒，让改动可控且可用 git review。

```bash
rexos agent run --workspace /path/to/repo --prompt "把 Foo 重命名成 Bar，并保持测试通过"
```

## 3) Provider 切换实验

逻辑不变，只改 `~/.rexos/config.toml` 的 provider/routing。

## 4) 本地小模型先跑通

先用 Ollama 小模型验证工具调用和 harness 流程，再切到更强的云端模型跑大任务。

