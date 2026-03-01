# 浏览器自动化（Playwright）

当 `web_fetch` 不够用（JS 渲染页面、多步交互、表单填写、需要截图留证）时，使用 `browser_*` 工具更可靠。

## 前置条件

安装 Playwright（Python）：

```bash
python3 -m pip install playwright
python3 -m playwright install chromium
```

如果你的 Python 可执行文件不是 `python3`，可以通过环境变量 `REXOS_BROWSER_PYTHON` 指定（例如 `python`）。

## 工具集

- `browser_navigate`：打开 URL（默认带 SSRF 防护）
- `browser_click`：按 CSS selector 点击（会做尽力的可见文本 fallback）
- `browser_type`：填写输入框
- `browser_read_page`：返回 `{title,url,content}`（content 会被截断）
- `browser_screenshot`：把 PNG 写入 workspace 相对路径
- `browser_close`：关闭 session（可重复调用）

## 推荐循环

1. `browser_navigate` 打开入口页面
2. `browser_read_page` 确认状态
3. 每次只做一个小动作：`browser_click` 或 `browser_type`
4. 再次 `browser_read_page` 确认页面确实变化
5. 直到完成，最后 `browser_screenshot` 留证并 `browser_close`

## Selector 小技巧

尽量使用稳定属性，而不是容易变化的文案文本：

- `#id`
- `[name="q"]`
- `[data-testid="submit"]`
- `button[type="submit"]`

如果 CSS selector 失败，`browser_click` 会尝试 **尽力的可见文本 fallback**。尽量写得具体，避免“OK / 确定”这种歧义文本导致点错。

## Prompt 模板（可直接复制）

```text
你可以使用 RexOS 的 browser 工具（browser_navigate/click/type/read_page/screenshot/close）。

规则：
- navigate/click/type 之后必须立刻 browser_read_page，先验证页面状态再做下一步。
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
- 报错提示 session 未启动：先调用 `browser_navigate`。
