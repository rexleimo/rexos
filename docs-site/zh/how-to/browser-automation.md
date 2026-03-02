# 浏览器自动化（Playwright）

当 `web_fetch` 不够用（JS 渲染页面、多步交互、表单填写、需要截图留证）时，使用 `browser_*` 工具更可靠。

更多可复制粘贴的配方见：[浏览器案例](browser-use-cases.md)。

## 前置条件

安装 Playwright（Python）：

```bash
python3 -m pip install playwright
python3 -m playwright install chromium
```

如果你的 Python 可执行文件不是 `python3`，可以通过环境变量 `REXOS_BROWSER_PYTHON` 指定（例如 `python`）。

## Headless / 有界面

RexOS 默认以 **headless** 模式启动 Chromium。

如果你想看到浏览器窗口（本地调试 / 演示），在第一次 `browser_navigate` 时设置 `headless=false`：

```json
{ "url": "https://www.baidu.com", "headless": false }
```

你也可以设置 `REXOS_BROWSER_HEADLESS=0`，让未显式传入 `headless` 时默认使用有界面模式。

## 工具集

- `browser_navigate`：打开 URL（默认带 SSRF 防护）
- `browser_click`：按 CSS selector 点击（会做尽力的可见文本 fallback）
- `browser_type`：填写输入框
- `browser_press_key`：按键（例如用 `Enter` 提交表单）
- `browser_wait_for`：等待 selector/text 出现
- `browser_read_page`：返回 `{title,url,content}`（content 会被截断）
- `browser_screenshot`：把 PNG 写入 workspace 相对路径
- `browser_close`：关闭 session（可重复调用）

## 推荐循环

1. `browser_navigate` 打开入口页面
2. `browser_read_page` 确认状态
3. 每次只做一个小动作：`browser_click` 或 `browser_type`
   - 需要提交表单时，用 `browser_press_key` 按 `Enter`。
4. 如果页面是异步更新，用 `browser_wait_for`（selector/text）等待新状态出现
5. 再次 `browser_read_page` 确认页面确实变化
6. 直到完成，最后 `browser_screenshot` 留证并 `browser_close`

## Selector 小技巧

尽量使用稳定属性，而不是容易变化的文案文本：

- `#id`
- `[name="q"]`
- `[data-testid="submit"]`
- `button[type="submit"]`

如果 CSS selector 失败，`browser_click` 会尝试 **尽力的可见文本 fallback**。尽量写得具体，避免“OK / 确定”这种歧义文本导致点错。

## Prompt 模板（可直接复制）

```text
你可以使用 RexOS 的 browser 工具（browser_navigate/click/type/press_key/wait_for/read_page/screenshot/close）。

规则：
- navigate/click/type/press_key 之后尽快 browser_read_page；如果页面异步更新，先 browser_wait_for 再 read_page。
- 动作尽量少且可回滚。selector 失败时先读页面内容，再调整 selector。
- 最后把截图保存到 .rexos/browser/<topic>.png。
- 未经用户明确确认，不要输入账号密码，也不要进行任何付费/下单操作。
```

示例运行：

```bash
rexos agent run --workspace . --prompt "使用 browser 工具打开 https://example.com，读取页面内容，把简短总结写到 notes/example.md，并把截图保存到 .rexos/browser/example.png。"
```

## 安全说明

- `browser_navigate` 默认拒绝 loopback/private 目标；只有本地/私网测试才建议显式开启 `allow_private=true`。
- `browser_screenshot` 只允许写到 workspace 相对路径（不允许绝对路径、不允许 `..`、不允许通过 symlink 逃逸）。

## 故障排查

- 报错提示 Playwright 缺失：按“前置条件”安装。
- 报错提示找不到 `python3`：设置 `REXOS_BROWSER_PYTHON=python`。
- 看不到浏览器窗口：默认是 headless；用 `headless=false`（或设置 `REXOS_BROWSER_HEADLESS=0`）。
- 报错提示 session 未启动：先调用 `browser_navigate`。
