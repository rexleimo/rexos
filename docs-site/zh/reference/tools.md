# 工具参考

RexOS 对 agent runtime 暴露了一组小而清晰的核心工具集，并提供一层“兼容工具面”（别名 + 预留名称），便于复用遵循常见工具约定的 prompts / manifests。

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

## `browser_*`（Playwright）

浏览器工具通过 Python Playwright bridge 提供无头浏览器自动化能力：

- `browser_navigate` / `browser_click` / `browser_type` / `browser_read_page` / `browser_screenshot` / `browser_close`

说明：

- `browser_navigate` 默认带 SSRF 防护（拒绝 loopback/private 目标，除非 `allow_private=true`）。
- `browser_screenshot` 只允许写入 workspace 相对路径（不允许绝对路径、不允许 `..`、不允许通过 symlink 逃逸）。
- 需要 Python + Playwright：

  ```bash
  python3 -m pip install playwright
  python3 -m playwright install chromium
  ```

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
- `task_post` / `task_claim` / `task_complete` / `task_list`
- `event_publish`
- `schedule_create` / `schedule_list` / `schedule_delete`
- `cron_create` / `cron_list` / `cron_cancel`
- `knowledge_add_entity` / `knowledge_add_relation` / `knowledge_query`

## 预留工具（stubs）

以下工具名已定义，但当前会直接返回 `tool not implemented yet: <name>`：

`channel_send`,
`hand_list`, `hand_activate`, `hand_status`, `hand_deactivate`,
`a2a_discover`, `a2a_send`,
`text_to_speech`, `speech_to_text`,
`docker_exec`,
`process_start`, `process_poll`, `process_write`, `process_kill`, `process_list`,
`canvas_present`。
