# 工具参考

RexOS 对 agent runtime 暴露了一组小而清晰的核心工具集，并提供一层“兼容工具面”（别名 + 预留名称），便于复用遵循常见工具约定的 prompts / manifests。

## 工具索引（60+）

你截图里看到的“目录（Table of contents）”只列出**章节标题**，并不会把每一个工具名都展开。很多工具会按模式归类，比如 `browser_*`、`process_*`、`agent_*` 等。

写 prompt / manifest 需要精确工具名时，用下面这个索引：

### 核心

`fs_read`, `fs_write`, `shell`, `web_fetch`, `pdf`, `pdf_extract`

### 浏览器

`browser_navigate`, `browser_back`, `browser_scroll`, `browser_click`, `browser_type`, `browser_press_key`, `browser_wait`, `browser_wait_for`, `browser_read_page`, `browser_run_js`, `browser_screenshot`, `browser_close`

### 兼容别名

`file_read`, `file_write`, `file_list`, `apply_patch`, `shell_exec`, `web_search`, `memory_store`, `memory_recall`

### 多媒体

`image_analyze`, `image_generate`, `location_get`, `media_describe`, `media_transcribe`, `speech_to_text`, `text_to_speech`

### A2A

`a2a_discover`, `a2a_send`

### 沙盒与进程

`docker_exec`, `process_start`, `process_poll`, `process_write`, `process_kill`, `process_list`, `canvas_present`

### Runtime 协作与调度

`agent_spawn`, `agent_list`, `agent_find`, `agent_send`, `agent_kill`, `hand_list`, `hand_activate`, `hand_status`, `hand_deactivate`, `task_post`, `task_claim`, `task_complete`, `task_list`, `event_publish`, `schedule_create`, `schedule_list`, `schedule_delete`, `cron_create`, `cron_list`, `cron_cancel`, `channel_send`, `knowledge_add_entity`, `knowledge_add_relation`, `knowledge_query`

## `fs_read`

读取 **相对于 workspace root** 的 UTF-8 文本文件。

- 拒绝绝对路径
- 拒绝 `..` 目录穿越
- 拒绝 symlink 逃逸

## `fs_write`

写入 **相对于 workspace root** 的 UTF-8 文本文件（必要时创建父目录）。

沙盒规则与 `fs_read` 相同。

## `shell`

在 workspace 内执行 shell 命令：

- Unix：通过 `bash -c`
- Windows：通过 PowerShell

RexOS 会强制超时，并使用尽量最小的环境。

## `web_fetch`

抓取一个 HTTP(S) URL，并返回一小段响应体。

默认拒绝 loopback/private IP 段（基础 SSRF 防护）。本地测试可用 `allow_private=true` 显式放开。

当 `truncated=true` 时，RexOS 会返回 **head+tail** 片段，并在中间插入 `[...] middle omitted [...]` 标记，同时返回 `bytes`（实际返回）与 `total_bytes`（原始大小）。

## `pdf`

从 workspace 内的 PDF 文件提取文本（best-effort）。

参数：

- `path`（必填）：workspace 相对路径的 `.pdf`
- `pages`（可选）：页码选择器（从 1 开始），例如 `"1"`、`"1-3"`、`"2,4-6"`
- `max_pages`（可选）：默认 10，最大 50
- `max_chars`（可选）：默认 12000，最大 50000

返回 JSON：

- `path`
- `text`（可能被截断）
- `truncated`（bool）
- `bytes`（文件大小）
- `pages_total`
- `pages`（选择器字符串，或 null）
- `pages_extracted`

## `browser_*`（CDP）

浏览器工具默认通过 **Chrome DevTools Protocol（CDP）** 提供无头浏览器自动化能力（无需 Python）：

- `browser_navigate` / `browser_back` / `browser_scroll` / `browser_click` / `browser_type` / `browser_press_key` / `browser_wait` / `browser_wait_for` / `browser_read_page` / `browser_run_js` / `browser_screenshot` / `browser_close`

说明：

- `browser_navigate` 默认带 SSRF 防护（拒绝 loopback/private 目标，除非 `allow_private=true`）。
- 默认是 headless。如需显示浏览器窗口，可在 `browser_navigate` 传 `headless=false`（或设置 `REXOS_BROWSER_HEADLESS=0` 作为默认值）。
- `browser_screenshot` 只允许写入 workspace 相对路径（不允许绝对路径、不允许 `..`、不允许通过 symlink 逃逸）。
- 默认后端是 CDP，需要本机存在 Chromium 系浏览器（Chrome/Chromium/Edge）。如果 RexOS 找不到可执行文件，设置 `REXOS_BROWSER_CHROME_PATH`。
- 可选远程 CDP：设置 `REXOS_BROWSER_CDP_HTTP`（例如 `http://127.0.0.1:9222`）。
- 可选远程 tab 选择：设置 `REXOS_BROWSER_CDP_TAB_MODE=reuse` 以跳过 `/json/new` 并复用已有 page target（默认：`new`）。
- 对 loopback CDP HTTP（`127.0.0.1` / `localhost`），RexOS 会绕过代理设置，避免企业代理误配置导致本地自动化失效。
- 可选 legacy 后端（Playwright bridge）：设置 `REXOS_BROWSER_BACKEND=playwright` 并安装 Python + Playwright：

  ```bash
  python3 -m pip install playwright
  python3 -m playwright install chromium
  ```

`browser_wait` 是一个“只等 selector”的兼容工具。需要等待 selector 或文本时，优先用 `browser_wait_for`。

`browser_run_js` 适用于 selector 不好写时抽取结构化字段（例如某个 heading）。在不可信页面上使用要谨慎。

## 兼容别名

以下工具名用于兼容常见工具约定，实际会映射到 RexOS 内置工具：

- `file_read` → `fs_read`
- `file_write` → `fs_write`
- `file_list` → 目录列表（workspace 相对路径；允许 `.`）
- `shell_exec` → `shell`
- `apply_patch` → 应用 `*** Begin Patch` / `*** End Patch` 格式的补丁（add/update/delete）
- `web_search` → DuckDuckGo HTML 搜索（best-effort；返回简短文本列表）
- `memory_store` / `memory_recall` → 共享 KV（持久化在 `~/.rexos/rexos.db`）

## `image_analyze`

分析 workspace 内的图片文件，并返回基础元数据 JSON（`format`、`width`、`height`、`bytes`）。

当前支持：PNG、JPEG、GIF。

## `location_get`

返回环境元数据 JSON（`os`、`arch`、`tz`、`lang`）。

RexOS 不会做基于 IP 的地理定位推断。

## `media_describe`

描述 workspace 内的媒体文件，并返回 best-effort 元数据 JSON（`kind`、`bytes`、`ext`）。

## `media_transcribe`

将媒体转成文本。

当前仅支持读取 workspace 内的 **文本转写/字幕文件**（`.txt`、`.md`、`.srt`、`.vtt`），并返回 JSON（`text`）。

## `image_generate`

根据 prompt 生成图片资产。

当前仅支持输出 **SVG**，写入 workspace 相对路径 `path`（建议使用 `.svg` 文件名）。

## Runtime 协作与调度工具

以下工具由 agent runtime 实现（不是独立的 `Toolset`），状态会持久化到 `~/.rexos/rexos.db`：

- `agent_spawn` / `agent_list` / `agent_find` / `agent_send` / `agent_kill`
- `hand_list` / `hand_activate` / `hand_status` / `hand_deactivate`
- `task_post` / `task_claim` / `task_complete` / `task_list`
- `event_publish`
- `schedule_create` / `schedule_list` / `schedule_delete`
- `cron_create` / `cron_list` / `cron_cancel`
- `channel_send`（写入 outbox；用 `rexos channel drain` 投递）
- `knowledge_add_entity` / `knowledge_add_relation` / `knowledge_query`

## `channel_send`

将一条外发消息写入 outbox。实际投递由 dispatcher 在带副作用的进程中完成：

- 单次执行：`rexos channel drain`
- 常驻 worker：`rexos channel worker`

当前支持的 channel：

- `console`：drain 时打印到 stdout
- `webhook`：POST JSON 到 `REXOS_WEBHOOK_URL`

## `hand_*`

Hands 是一组小而精的“agent 模板”，用于快速启动一个专用 agent 实例。

- `hand_list`：列出内置 Hands 以及是否处于 active 状态
- `hand_activate`：激活某个 Hand，返回 `{instance_id, agent_id, ...}`
- `hand_status`：查询某个 `hand_id` 当前是否有 active 实例
- `hand_deactivate`：按 `instance_id` 停用实例（会 kill 对应的底层 agent）

`hand_activate` 后可用 `agent_send` 与返回的 `agent_id` 交互。

## `a2a_*`

A2A 工具用于与外部 A2A 兼容 agent 交互：

- `a2a_discover`：抓取 `/.well-known/agent.json` 的 agent card
- `a2a_send`：向 A2A endpoint 发送 JSON-RPC `tasks/send`

默认带 SSRF 防护；本地测试可用 `allow_private=true` 显式放开。

## `speech_to_text`

将媒体转成文本。

MVP 行为：支持 **文本转写/字幕文件**（`.txt`、`.md`、`.srt`、`.vtt`），返回 JSON（`transcript` 与 `text`）。

## `text_to_speech`

将文本转换为音频文件。

MVP 行为：在 workspace 内写出一个短 `.wav` 文件（作为真实 TTS 的占位实现）。

## `docker_exec`

在一次性 Docker 容器内执行命令（会挂载 workspace）。

- 默认禁用：设置 `REXOS_DOCKER_EXEC_ENABLED=1`
- 可选镜像：`REXOS_DOCKER_EXEC_IMAGE`（默认 `alpine:3.20`）

## `process_*`

启动并与长运行进程交互：

- `process_start` / `process_poll` / `process_write` / `process_kill` / `process_list`

进程以 workspace 为工作目录，并使用尽量最小的环境。

`process_poll` 返回 JSON：

- `stdout` / `stderr`（增量输出）
- `stdout_truncated` / `stderr_truncated`（bool；为 true 时输出为 head+tail 片段，中间带 `[...] middle omitted [...]`）
- `exit_code`（存活时为 null）
- `alive`（bool）

## `canvas_present`

将一段经过清洗的 HTML 保存到 workspace（`output/` 目录下），并返回元数据（`saved_to`、`canvas_id` 等）。

会拒绝脚本、事件处理器属性（如 `onclick=`）、以及 `javascript:` URL。
