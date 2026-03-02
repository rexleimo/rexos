# 概念

RexOS 面向“一个 prompt 不够”的长任务工作流。

## Workspace（工作目录）

大多数 RexOS 命令都基于一个 **workspace 目录**：

- 工具调用会被沙盒限制在该目录内（文件系统 + shell 工作目录）
- harness 的持久化产物也存放在该目录里

## Memory（SQLite 持久化记忆）

RexOS 会把以下信息持久化到 `~/.rexos/rexos.db`：

- sessions
- 对话消息
- 小型 key/value 状态

因此你可以在后续多次运行中继续推进任务。

## Tools（工具沙盒）

agent 可以调用工具，例如：

- `fs_read` / `fs_write`（仅 workspace 内，阻止 `..`）
- `shell`（仅 workspace 内）
- `web_fetch`（默认 SSRF 防护）
- `browser_*`（默认通过 CDP 进行无头浏览器自动化；Playwright 可选）

!!! note "Browser 工具前置条件"
    `browser_*` 默认通过 **CDP** 驱动本机 Chromium 系浏览器（Chrome/Chromium/Edge），无需 Python。

    如果 RexOS 找不到浏览器可执行文件，请设置 `REXOS_BROWSER_CHROME_PATH`。

    可选 legacy 后端：设置 `REXOS_BROWSER_BACKEND=playwright`，并安装 Python + Playwright：

    ```bash
    python3 -m pip install playwright
    python3 -m playwright install chromium
    ```

## 模型路由（Model routing）

RexOS 会把一次 run 归类为任务类型：

- planning
- coding
- summary

每种类型可以路由到不同 provider/model。

## Harness（durable 长任务）

harness 在此基础上加了一层“可持续推进”的工作流：

1. 初始化 workspace 并生成持久化产物
2. 多次增量运行
3. 通过 `init.sh` / `init.ps1` 做验证
4. 通过 git commits 做 checkpoint
