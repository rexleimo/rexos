# Harness 长任务

Harness 适合“一个上下文窗口放不下”的任务。它把进度做成 **durable**：

- workspace 目录里的持久化产物（`features.json`、`rexos-progress.md`、init scripts）
- 验证脚本（Unix: `init.sh`；Windows: `init.ps1`）
- git commits 作为 checkpoint
- 每个 workspace 持久化的 session id

## 1) 创建 workspace

先准备一个空目录给本教程使用：

=== "macOS/Linux"
    ```bash
    mkdir -p rexos-task
    ```

=== "Windows (PowerShell)"
    ```powershell
    mkdir rexos-task
    ```

## 2) 初始化 Harness

无 prompt：只创建产物 + 初始 git commit

```bash
rexos harness init rexos-task
```

带 prompt：会运行 initializer agent 去生成 `features.json` 并调整 init script

```bash
rexos harness init rexos-task --prompt "在这个 workspace 里创建一个小 CLI，并保证测试通过"
```

## 3) 跑一次增量 session

```bash
rexos harness run rexos-task --prompt "实现下一个 feature"
```

Harness 会：preflight → agent → 跑 init script → 失败重试/成功 checkpoint。
