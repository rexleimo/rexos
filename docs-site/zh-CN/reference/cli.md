# CLI 参考

RexOS 提供单个二进制：`rexos`。

## 顶层命令

- `rexos init` — 初始化 `~/.rexos`（配置 + 数据库）
- `rexos agent run` — 在 workspace 中运行一次 agent session
- `rexos channel drain` — 执行一次 outbox drain（投递队列中的消息）
- `rexos channel worker` — 运行轮询 outbox 的 dispatcher
- `rexos harness init` — 初始化 harness workspace（持久化产物 + git）
- `rexos harness run` — 运行一次增量 harness session
- `rexos daemon start` — 启动 HTTP daemon

## 示例

=== "macOS/Linux"
    ```bash
    rexos init

    mkdir -p rexos-work
    rexos agent run --workspace rexos-work --prompt "Create hello.txt"

    mkdir -p rexos-task
    rexos harness init rexos-task --prompt "Initialize a features checklist for refactoring this repo"
    rexos harness run rexos-task --prompt "Continue"

    rexos channel drain

    rexos daemon start --addr 127.0.0.1:8787
    ```

=== "Windows (PowerShell)"
    ```powershell
    rexos init

    mkdir rexos-work
    rexos agent run --workspace rexos-work --prompt "Create hello.txt"

    mkdir rexos-task
    rexos harness init rexos-task --prompt "Initialize a features checklist for refactoring this repo"
    rexos harness run rexos-task --prompt "Continue"

    rexos channel drain

    rexos daemon start --addr 127.0.0.1:8787
    ```
