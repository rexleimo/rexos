# 安全与沙盒

RexOS 的设计核心是：在允许 LLM 驱动工具调用的同时，尽量加上可解释的护栏。

## Workspace 沙盒

文件系统工具：

- 只允许 **相对路径**（相对于 workspace root）
- 拒绝 `..` 目录穿越
- 拒绝通过 symlink 逃逸 workspace

## Shell 工具

shell 工具：

- 在 workspace 内运行
- 环境尽量最小化
- 强制超时

Windows 下使用 PowerShell；Unix 下使用 bash。

## Web fetch（SSRF 防护）

`web_fetch` 默认拒绝访问 loopback/private IP 段，降低 SSRF 风险。

本地测试场景下，你可以显式开启 `allow_private=true` 来允许访问私网目标。

## 浏览器工具（browser_*）

RexOS 默认通过 **CDP** 启动并控制无头浏览器（无需 Python），也支持 legacy 的 Playwright bridge 后端。

- `browser_navigate` / `browser_click` / `browser_type` / `browser_press_key` / `browser_wait_for` / `browser_read_page` / `browser_screenshot` / `browser_close`

安全说明：

- `browser_navigate` 默认与 `web_fetch` 类似的 SSRF 防护（拒绝 loopback/private 目标，除非 `allow_private=true`）。
- `browser_read_page` 与 `browser_screenshot` 也会做同样的 SSRF 防护（除非你开启了 `allow_private`）。
- `browser_screenshot` 只会写入 **workspace 相对路径**（不允许绝对路径、不允许 `..`、不允许通过 symlink 逃逸）。

## 未来：审批（Approvals）

RexOS 的结构允许未来加入“审批钩子”，对更高风险的行为（网络写入、破坏性命令等）进行确认。
