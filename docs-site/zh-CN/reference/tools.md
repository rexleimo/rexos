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

`agent_spawn`, `agent_list`, `agent_find`, `agent_send`, `agent_kill`, `hand_list`, `hand_activate`, `hand_status`, `hand_deactivate`, `task_post`, `task_claim`, `task_complete`, `task_list`, `event_publish`, `schedule_create`, `schedule_list`, `schedule_delete`, `cron_create`, `cron_list`, `cron_cancel`, `channel_send`, `workflow_run`, `knowledge_add_entity`, `knowledge_add_relation`, `knowledge_query`

## 运行这些示例

下面的示例多数按这种格式写：

- 一段 **tool call** JSON（即工具参数对象）
- 一段可直接复制的 **prompt**

快速运行方式：

=== "Bash（macOS/Linux）"
    ```bash
    rexos agent run --workspace . --prompt "<粘贴 prompt>"
    ```

=== "PowerShell（Windows）"
    ```powershell
    rexos agent run --workspace . --prompt "<粘贴 prompt>"
    ```

## `fs_read`

读取 **相对于 workspace root** 的 UTF-8 文本文件。

- 拒绝绝对路径
- 拒绝 `..` 目录穿越
- 拒绝 symlink 逃逸

### 示例

工具调用：

```json
{ "path": "README.md" }
```

Prompt：

```text
使用 fs_read 读取 README.md，然后写 notes/readme_summary.md（5 条 bullet 总结）。
```

## `fs_write`

写入 **相对于 workspace root** 的 UTF-8 文本文件（必要时创建父目录）。

沙盒规则与 `fs_read` 相同。

### 示例

工具调用：

```json
{ "path": "notes/hello.md", "content": "Hello from RexOS\\n" }
```

Prompt：

```text
使用 fs_write 创建 notes/hello.md，内容包含一行问候语和今天日期。
```

## `shell`

在 workspace 内执行 shell 命令：

- Unix：通过 `bash -c`
- Windows：通过 PowerShell

RexOS 会强制超时，并使用尽量最小的环境。

### 示例

工具调用：

```json
{ "command": "echo READY && ls" }
```

Prompt：

```text
使用 shell 执行安全命令（echo READY 并列出 workspace）。把完整输出写到 notes/shell_output.txt。
```

## `web_fetch`

抓取一个 HTTP(S) URL，并返回一小段响应体。

默认拒绝 loopback/private IP 段（基础 SSRF 防护）。本地测试可用 `allow_private=true` 显式放开。

当 `truncated=true` 时，RexOS 会返回 **head+tail** 片段，并在中间插入 `[...] middle omitted [...]` 标记，同时返回 `bytes`（实际返回）与 `total_bytes`（原始大小）。

### 示例

工具调用：

```json
{ "url": "https://example.com", "timeout_ms": 20000, "max_bytes": 200000 }
```

Prompt：

```text
使用 web_fetch 抓取 https://example.com，然后写 notes/web_fetch_example.md：包含 status、content_type、以及 body 的前 200 个字符。
```

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

### 示例

工具调用：

```json
{ "path": "samples/dummy.pdf", "pages": "1-2", "max_pages": 10, "max_chars": 12000 }
```

Prompt：

```text
使用 pdf（或 pdf_extract）从 samples/dummy.pdf 提取文本（pages=1-2）。然后写 notes/pdf_excerpt.md：包含 (1) 6 条 bullet 总结，(2) 关键词/术语，(3) 你观察到的缺失/乱码/异常段落。只能基于提取到的 PDF 文本，不要编造内容。
```

另见：[PDF 总结（case task）](../examples/case-tasks/pdf-summarize.md)。

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

### `browser_navigate`

启动（或复用）一个浏览器会话并跳转到 URL。

工具调用：

```json
{ "url": "https://example.com", "timeout_ms": 30000, "headless": false }
```

Prompt：

```text
使用 browser_navigate 打开 https://example.com（headless=false）。然后保存截图到 .rexos/browser/example.png，最后 browser_close 关闭浏览器。
```

另见：[有界面 smoke check](../how-to/browser-use-cases/gui-smoke-check.md)，[百度今天天气（Ollama）](../how-to/browser-use-cases/baidu-weather.md)。

### `browser_back`

回到上一页（需要已启动浏览器会话）。

工具调用：

```json
{}
```

Prompt：

```text
使用 browser_navigate 依次打开 https://example.com 和 https://www.iana.org/domains/reserved，然后调用 browser_back，并确认 URL 回到 example.com。
```

### `browser_scroll`

滚动页面（需要已启动浏览器会话）。

工具调用：

```json
{ "direction": "down", "amount": 800 }
```

Prompt：

```text
使用 browser_navigate 打开 https://example.com，然后调用 browser_scroll 向下滚动 800，再截图到 .rexos/browser/scroll.png，最后关闭浏览器。
```

### `browser_click`

按 CSS selector 点击元素（best-effort：也会尝试按文本匹配链接/按钮）。

工具调用：

```json
{ "selector": "More information" }
```

Prompt：

```text
使用 browser_navigate 打开 https://example.com，然后 browser_click 点击 \"More information\"，截图到 .rexos/browser/click.png，最后关闭浏览器。
```

### `browser_type`

向输入框输入文本（需要已启动浏览器会话）。

工具调用：

```json
{ "selector": "input[name=\"wd\"]", "text": "北京 今天天气" }
```

Prompt：

```text
使用 browser_navigate 打开 https://www.baidu.com。等待 input[name=\"wd\"] 出现后，browser_type 输入 \"北京 今天天气\"，再按 Enter 提交，截图并关闭浏览器。
```

另见：[百度今天天气（Ollama）](../how-to/browser-use-cases/baidu-weather.md)。

### `browser_press_key`

发送按键（可选先 focus 某个 selector）。

工具调用：

```json
{ "selector": "input[name=\"wd\"]", "key": "Enter" }
```

Prompt：

```text
在搜索框输入后，用 browser_press_key（key=Enter）提交搜索；如果站点阻止自动化，回退为直接打开搜索结果 URL。
```

### `browser_wait`

等待 selector（兼容工具；只支持 selector）。

工具调用：

```json
{ "selector": "#content_left", "timeout_ms": 30000 }
```

Prompt：

```text
用 browser_wait 等待结果容器出现，然后再 read_page 提取文本。
```

### `browser_wait_for`

等待 selector **或** 文本子串。

工具调用：

```json
{ "selector": "#content_left", "text": "天气", "timeout_ms": 30000 }
```

Prompt：

```text
用 browser_wait_for 等到 #content_left 出现或页面包含 \"天气\"。然后 browser_read_page 读页面并做 3 条 bullet 总结。
```

### `browser_read_page`

提取可见文本与基础元数据（title/url）。

工具调用：

```json
{}
```

Prompt：

```text
在浏览器跳转后，调用 browser_read_page，并把 content 的前 2000 字写入 notes/page.txt。
```

### `browser_run_js`

运行一段 JS expression 并返回结果（谨慎使用）。

工具调用：

```json
{ "expression": "document.title" }
```

Prompt：

```text
使用 browser_run_js 读取 document.title，然后写入 notes/title.txt。
```

### `browser_screenshot`

保存 PNG 截图到 workspace（默认路径：`.rexos/browser/screenshot.png`）。

工具调用：

```json
{ "path": ".rexos/browser/page.png" }
```

Prompt：

```text
在关键步骤后调用 browser_screenshot，把证据保存到 .rexos/browser/page.png。
```

### `browser_close`

关闭浏览器会话（可重复调用；建议每个浏览器流程末尾都调用一次）。

工具调用：

```json
{}
```

Prompt：

```text
在任何浏览器流程结束时调用 browser_close 清理资源。
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

### `file_read`

工具调用：

```json
{ "path": "README.md" }
```

Prompt：

```text
使用 file_read 读取 README.md，然后写 notes/readme_summary.md（5 条 bullet 总结）。
```

### `file_write`

工具调用：

```json
{ "path": "notes/hello.txt", "content": "hello\\n" }
```

Prompt：

```text
使用 file_write 创建 notes/hello.txt，写入一行短消息。
```

### `file_list`

工具调用：

```json
{ "path": "." }
```

Prompt：

```text
使用 file_list 列出 workspace 根目录文件，然后写 notes/files.md（把 listing 原样写进去）。
```

### `shell_exec`

工具调用：

```json
{ "command": "echo hi", "timeout_seconds": 60 }
```

Prompt：

```text
使用 shell_exec 执行一个安全命令，并把输出写入 notes/shell_exec.txt。
```

### `apply_patch`

工具调用：

```json
{
  "patch": "*** Begin Patch\\n*** Add File: notes/patched.txt\\n+hello from apply_patch\\n*** End Patch\\n"
}
```

Prompt：

```text
使用 apply_patch 新增 notes/patched.txt（1 行文本），然后用 fs_read 读取它来确认成功。
```

### `web_search`

工具调用：

```json
{ "query": "RexOS harness 长任务", "max_results": 5 }
```

Prompt：

```text
使用 web_search 搜索 \"RexOS harness 长任务\"，取 5 条结果，并写 notes/search.md（标题 + URL）。
```

### `memory_store`

工具调用：

```json
{ "key": "demo.favorite_color", "value": "blue" }
```

Prompt：

```text
使用 memory_store 保存 demo.favorite_color=blue，然后用 memory_recall 取回，并把结果写到 notes/memory.md。
```

### `memory_recall`

工具调用：

```json
{ "key": "demo.favorite_color" }
```

Prompt：

```text
使用 memory_recall 读取 demo.favorite_color，并打印/写入 notes/memory_value.txt。
```

## `image_analyze`

分析 workspace 内的图片文件，并返回基础元数据 JSON（`format`、`width`、`height`、`bytes`）。

当前支持：PNG、JPEG、GIF。

### 示例

工具调用：

```json
{ "path": ".rexos/browser/page.png" }
```

Prompt：

```text
对 .rexos/browser/page.png 调用 image_analyze，并把返回 JSON 写到 notes/image_meta.json。
```

## `location_get`

返回环境元数据 JSON（`os`、`arch`、`tz`、`lang`）。

RexOS 不会做基于 IP 的地理定位推断。

### 示例

工具调用：

```json
{}
```

Prompt：

```text
调用 location_get，并把返回 JSON 写到 notes/env.json。
```

## `media_describe`

描述 workspace 内的媒体文件，并返回 best-effort 元数据 JSON（`kind`、`bytes`、`ext`）。

### 示例

工具调用：

```json
{ "path": "notes/readme_summary.md" }
```

Prompt：

```text
对 notes/readme_summary.md 调用 media_describe，并把返回 JSON 写到 notes/media_meta.json。
```

## `media_transcribe`

将媒体转成文本。

当前仅支持读取 workspace 内的 **文本转写/字幕文件**（`.txt`、`.md`、`.srt`、`.vtt`），并返回 JSON（`text`）。

### 示例

工具调用：

```json
{ "path": "samples/transcript.txt" }
```

Prompt：

```text
先用 fs_write 创建 samples/transcript.txt（写 3 行短对话），再用 media_transcribe 读取它，并把返回 text 写到 notes/transcript.md。
```

## `image_generate`

根据 prompt 生成图片资产。

当前仅支持输出 **SVG**，写入 workspace 相对路径 `path`（建议使用 `.svg` 文件名）。

### 示例

工具调用：

```json
{ "prompt": "一个写着 RexOS 的简洁 SVG 徽章", "path": "assets/rexos_badge.svg" }
```

Prompt：

```text
使用 image_generate 生成 assets/rexos_badge.svg。然后用 fs_read 读取该文件，并把前 20 行写到 notes/badge_preview.md。
```

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

### `agent_spawn`

创建一个 agent 会话记录（会持久化），并返回其信息。

工具调用：

```json
{ "name": "Helper", "system_prompt": "你是一个简洁的助手。" }
```

Prompt：

```text
使用 agent_spawn 创建一个名为 Helper 的 agent（给一个简短 system_prompt）。然后调用 agent_list，并把输出写到 notes/agents.json。
```

### `agent_list`

工具调用：

```json
{}
```

Prompt：

```text
调用 agent_list，并把返回 JSON 写到 notes/agents.json。
```

### `agent_find`

工具调用：

```json
{ "query": "helper" }
```

Prompt：

```text
使用 agent_find（query=helper）查找 agent，并把结果写到 notes/agent_find.json。
```

### `agent_send`

工具调用：

```json
{ "agent_id": "<agent_id>", "message": "用 3 条 bullet 总结 workspace 的 README。" }
```

Prompt：

```text
先用 agent_spawn 创建一个 agent，拿到返回的 agent_id；然后用 agent_send 让它总结 README（3 条 bullet），把回复写到 notes/agent_reply.md。
```

### `agent_kill`

工具调用：

```json
{ "agent_id": "<agent_id>" }
```

Prompt：

```text
用 agent_kill 标记一个 agent 为 killed，然后用 agent_list 确认状态变化。
```

### `task_post`

发布一个任务到共享任务板。

工具调用：

```json
{ "title": "Demo task", "description": "写 notes/task.md（包含一个短 checklist）" }
```

Prompt：

```text
使用 task_post 创建一个 Demo task，然后调用 task_list 并把输出写到 notes/tasks.json。
```

### `task_list`

工具调用：

```json
{ "status": "pending" }
```

Prompt：

```text
调用 task_list（status=pending）列出待处理任务，并把输出写到 notes/tasks_pending.json。
```

### `task_claim`

领取一个 pending 任务（默认领取第一个符合条件的任务）。

工具调用：

```json
{}
```

Prompt：

```text
先用 task_post 创建一个任务，然后调用 task_claim 领取它。把返回的 claimed task JSON 保存下来，并从中拿到 task_id 继续完成任务。
```

### `task_complete`

工具调用：

```json
{ "task_id": "<task_id>", "result": "done" }
```

Prompt：

```text
对已领取的任务调用 task_complete（result 写一个简短结果），然后用 task_list 确认它已完成。
```

### `event_publish`

向事件日志追加一条事件记录。

工具调用：

```json
{ "event_type": "demo.finished", "payload": { "ok": true } }
```

Prompt：

```text
使用 event_publish 发布 demo.finished 事件（payload={ok:true}），然后写 notes/event_done.md（描述你发布了什么）。
```

### `schedule_create`

存储一个 schedule 定义（当前是“存储定义”；是否会自动执行取决于你的 runner/daemon 集成）。

工具调用：

```json
{ "description": "Daily standup reminder", "schedule": "every day 09:30", "enabled": true }
```

Prompt：

```text
使用 schedule_create 创建一个每日提醒 schedule，然后 schedule_list 并把输出写到 notes/schedules.json。
```

### `schedule_list`

工具调用：

```json
{}
```

Prompt：

```text
调用 schedule_list，并把输出写到 notes/schedules.json。
```

### `schedule_delete`

工具调用：

```json
{ "id": "<schedule_id>" }
```

Prompt：

```text
先 schedule_create，再对返回的 id 调用 schedule_delete，然后用 schedule_list 确认已删除。
```

### `cron_create`

存储一个 cron job 定义（当前是“存储定义”；是否会自动执行取决于你的 runner/daemon 集成）。

工具调用：

```json
{
  "job_id": "job1",
  "name": "Job One",
  "schedule": { "kind": "every", "every_secs": 300 },
  "action": { "kind": "system_event", "text": "tick" },
  "one_shot": false
}
```

Prompt：

```text
使用 cron_create 存储一个 demo cron 定义，然后 cron_list 并把输出写到 notes/cron.json。（注意：这里只是存储定义，不会自动跑，除非你有 runner。）
```

### `cron_list`

工具调用：

```json
{}
```

Prompt：

```text
调用 cron_list，并把输出写到 notes/cron.json。
```

### `cron_cancel`

工具调用：

```json
{ "job_id": "job1" }
```

Prompt：

```text
先 cron_create（job_id=job1），再 cron_cancel（job_id=job1），最后 cron_list 确认已取消。
```

### `knowledge_add_entity`

向 knowledge store 写入一个实体。

工具调用：

```json
{ "name": "RexOS", "entity_type": "project", "properties": { "repo": "rexleimo/rexos" } }
```

Prompt：

```text
使用 knowledge_add_entity 添加一个实体 RexOS，然后 knowledge_query 搜索 RexOS，并把输出写到 notes/knowledge.json。
```

### `knowledge_add_relation`

在两个实体之间添加一条关系（边）。

工具调用：

```json
{
  "source": "RexOS",
  "relation": "inspires",
  "target": "meos",
  "properties": { "confidence": 0.8 }
}
```

Prompt：

```text
使用 knowledge_add_relation 添加 RexOS -> meos 的关系，然后 knowledge_query 搜索 RexOS 并写 notes/knowledge.json。
```

### `knowledge_query`

检索 entities/relations（best-effort 子串查询）。

工具调用：

```json
{ "query": "RexOS" }
```

Prompt：

```text
使用 knowledge_query 搜索 RexOS，并把输出写到 notes/knowledge.json。
```

## `channel_send`

将一条外发消息写入 outbox。实际投递由 dispatcher 在带副作用的进程中完成：

- 单次执行：`rexos channel drain`
- 常驻 worker：`rexos channel worker`

当前支持的 channel：

- `console`：drain 时打印到 stdout
- `webhook`：POST JSON 到 `REXOS_WEBHOOK_URL`

参数（工具调用 JSON）：

- `channel`（必填）：`console` | `webhook`
- `recipient`（必填）：`console` 可用 `"stdout"`；`webhook` 可用一个逻辑名称（实际 URL 由配置/环境决定）
- `subject`（可选）
- `message`（必填）

### 示例

工具调用：

```json
{ "channel": "console", "recipient": "stdout", "subject": "demo", "message": "Hello from RexOS" }
```

Prompt：

```text
使用 channel_send 入队一条 console 消息（recipient=stdout），内容是 \"Hello from RexOS\"。然后提示我运行 `rexos channel drain` 来投递。
```

## `workflow_run`

运行一个多步骤工作流，并把执行状态持久化到 `.rexos/workflows/<workflow_id>.json`。

参数（工具调用 JSON）：

- `workflow_id`（可选）：稳定 workflow id，便于重复执行。
- `name`（可选）：工作流名称。
- `steps`（必填）：步骤数组。
  - `tool`（必填）
  - `arguments`（可选对象，默认 `{}`）
  - `name`（可选）
  - `approval_required`（可选布尔）：当启用审批策略时，强制该步骤走审批门。
- `continue_on_error`（可选）：步骤失败后是否继续执行。

### 示例

工具调用：

```json
{
  "workflow_id": "wf_demo",
  "name": "write-note",
  "steps": [
    {
      "name": "write",
      "tool": "fs_write",
      "arguments": { "path": "notes/workflow.txt", "content": "hello" }
    }
  ]
}
```

Prompt：

```text
使用 workflow_run 执行一个步骤，把 notes/workflow.txt 写成 \"hello\"，然后返回工作流状态。
```

## `hand_*`

Hands 是一组小而精的“agent 模板”，用于快速启动一个专用 agent 实例。

- `hand_list`：列出内置 Hands 以及是否处于 active 状态
- `hand_activate`：激活某个 Hand，返回 `{instance_id, agent_id, ...}`
- `hand_status`：查询某个 `hand_id` 当前是否有 active 实例
- `hand_deactivate`：按 `instance_id` 停用实例（会 kill 对应的底层 agent）

`hand_activate` 后可用 `agent_send` 与返回的 `agent_id` 交互。

### `hand_list`

工具调用：

```json
{}
```

Prompt：

```text
调用 hand_list，并把输出写到 notes/hands.json。选择一个 available 的 hand id。
```

### `hand_activate`

工具调用：

```json
{ "hand_id": "researcher", "config": { "topic": "RexOS" } }
```

Prompt：

```text
使用 hand_activate 激活 researcher hand。然后用返回的 agent_id 调用 agent_send，让它 web_search \"RexOS\" 并总结 3 条 bullet。
```

### `hand_status`

工具调用：

```json
{ "hand_id": "researcher" }
```

Prompt：

```text
调用 hand_status 查看 researcher hand 是否 active，并把结果写到 notes/hand_status.json。
```

### `hand_deactivate`

工具调用：

```json
{ "instance_id": "<instance_id>" }
```

Prompt：

```text
先 hand_activate 启动一个 hand，然后用返回的 instance_id 调用 hand_deactivate。再用 hand_list 确认它不再是 active。
```

## `a2a_*`

A2A 工具用于与外部 A2A 兼容 agent 交互：

- `a2a_discover`：抓取 `/.well-known/agent.json` 的 agent card
- `a2a_send`：向 A2A endpoint 发送 JSON-RPC `tasks/send`

默认带 SSRF 防护；本地测试可用 `allow_private=true` 显式放开。

### `a2a_discover`

获取 A2A agent card（RexOS 会在给定 host 上请求 `/.well-known/agent.json`）。

工具调用：

```json
{ "url": "https://example.com", "allow_private": false }
```

Prompt：

```text
对一个已知支持 A2A 的站点调用 a2a_discover，并把返回 JSON 写到 notes/agent_card.json。
```

### `a2a_send`

向 A2A endpoint URL 发送消息（JSON-RPC `tasks/send`）。

工具调用：

```json
{ "agent_url": "http://127.0.0.1:8787/a2a", "message": "hello", "session_id": "demo", "allow_private": true }
```

Prompt：

```text
使用 a2a_send 与一个 A2A endpoint 交互，并把返回 result JSON 写到 notes/a2a_result.json。
```

## `speech_to_text`

将媒体转成文本。

MVP 行为：支持 **文本转写/字幕文件**（`.txt`、`.md`、`.srt`、`.vtt`），返回 JSON（`transcript` 与 `text`）。

### 示例

工具调用：

```json
{ "path": "samples/transcript.txt" }
```

Prompt：

```text
先用 fs_write 创建 samples/transcript.txt（写一段简短转写/对白），再调用 speech_to_text 读取它，并把返回 JSON 写到 notes/stt.json。
```

## `text_to_speech`

将文本转换为音频文件。

MVP 行为：在 workspace 内写出一个短 `.wav` 文件（作为真实 TTS 的占位实现）。

### 示例

工具调用：

```json
{ "text": "Hello from RexOS", "path": ".rexos/audio/tts.wav" }
```

Prompt：

```text
使用 text_to_speech 生成 .rexos/audio/tts.wav（内容：Hello from RexOS）。然后对该文件调用 media_describe，并把结果写到 notes/tts_meta.json。
```

## `docker_exec`

在一次性 Docker 容器内执行命令（会挂载 workspace）。

- 默认禁用：设置 `REXOS_DOCKER_EXEC_ENABLED=1`
- 可选镜像：`REXOS_DOCKER_EXEC_IMAGE`（默认 `alpine:3.20`）

### 示例

工具调用：

```json
{ "command": "echo hello-from-docker && ls -la" }
```

Prompt：

```text
如果你已启用 docker_exec，使用 docker_exec 在容器里执行一个安全命令，并把 exit_code/stdout/stderr 写到 notes/docker_exec.json。
```

## `process_*`

启动并与长运行进程交互：

- `process_start` / `process_poll` / `process_write` / `process_kill` / `process_list`

进程以 workspace 为工作目录，并使用尽量最小的环境。

`process_poll` 返回 JSON：

- `stdout` / `stderr`（增量输出）
- `stdout_truncated` / `stderr_truncated`（bool；为 true 时输出为 head+tail 片段，中间带 `[...] middle omitted [...]`）
- `exit_code`（存活时为 null）
- `alive`（bool）

### `process_start`

启动一个长运行进程并返回 `process_id`。

工具调用（macOS/Linux 示例）：

```json
{ "command": "bash", "args": ["-lc", "echo READY; read line; echo ECHO:$line; sleep 30"] }
```

Prompt：

```text
使用 process_start 启动一个会先输出 READY、再回显一行输入的进程。记录返回的 process_id，后续步骤会用到。
```

### `process_poll`

工具调用：

```json
{ "process_id": "<process_id>" }
```

Prompt：

```text
循环调用 process_poll，直到 stdout 包含 READY。然后继续。
```

### `process_write`

工具调用：

```json
{ "process_id": "<process_id>", "data": "hi" }
```

Prompt：

```text
看到 READY 后，用 process_write 写入 \"hi\"，再 poll 直到看到 ECHO:hi。
```

### `process_list`

工具调用：

```json
{}
```

Prompt：

```text
调用 process_list，并把输出写到 notes/processes.json（确认你的 process_id 在列表中）。
```

### `process_kill`

工具调用：

```json
{ "process_id": "<process_id>" }
```

Prompt：

```text
用 process_kill 结束进程，然后再 process_list 确认它不再出现。
```

## `canvas_present`

将一段经过清洗的 HTML 保存到 workspace（`output/` 目录下），并返回元数据（`saved_to`、`canvas_id` 等）。

会拒绝脚本、事件处理器属性（如 `onclick=`）、以及 `javascript:` URL。

### 示例

工具调用：

```json
{ "title": "Demo report", "html": "<h1>Hello</h1><p>Generated by RexOS.</p>" }
```

Prompt：

```text
使用 canvas_present 生成一个简单 HTML 报告（标题 + 3 条要点）。然后 fs_read saved_to 对应的文件，并把文件名写到 notes/report_path.txt。
```
