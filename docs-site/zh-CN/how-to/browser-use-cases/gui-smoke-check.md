# 有界面 smoke check（example.com）

**目标：** 验证 `browser_*` 端到端可用，并在 workspace 里留下证据文件。

另见：[浏览器自动化（CDP）](../browser-automation.md)。

## 运行

=== "Bash (macOS/Linux)"
    ```bash
    mkdir -p rexos-demo && cd rexos-demo
    export REXOS_BROWSER_HEADLESS=0

    rexos agent run --workspace . --prompt "使用 browser 工具打开 https://example.com，读取页面内容，把 3 条要点写到 notes/example.md，并把截图保存到 .rexos/browser/example.png，然后关闭浏览器。"
    ```

=== "PowerShell (Windows)"
    ```powershell
    mkdir rexos-demo -Force | Out-Null
    cd rexos-demo
    $env:REXOS_BROWSER_HEADLESS = "0"

    rexos agent run --workspace . --prompt "使用 browser 工具打开 https://example.com，读取页面内容，把 3 条要点写到 notes/example.md，并把截图保存到 .rexos/browser/example.png，然后关闭浏览器。"
    ```

## 方案 B：Docker GUI 沙盒（Chromium + noVNC）

如果你不想在本机安装 Chrome/Chromium，可以用 Docker 跑一个带 GUI 的 Chromium（沙盒），然后 RexOS 通过 CDP 附加上去。

1) 启动容器（在 RexOS repo 根目录执行）：

```bash
docker compose -f docker/sandbox-browser/compose.yml up --build
```

2) 打开 noVNC 观察界面：

- URL：`http://127.0.0.1:6080/vnc.html`
- 密码：`rexos`（见 `docker/sandbox-browser/compose.yml`）

3) 在另一个终端里，把 RexOS 的浏览器工具指向沙盒的 CDP：

=== "Bash (macOS/Linux)"
    ```bash
    export REXOS_BROWSER_CDP_HTTP="http://127.0.0.1:9222"
    ```

=== "PowerShell (Windows)"
    ```powershell
    $env:REXOS_BROWSER_CDP_HTTP = "http://127.0.0.1:9222"
    ```

然后按上面的提示运行同一个 smoke check prompt 即可。

## 预期结果

- `notes/example.md`
- `.rexos/browser/example.png`

## 故障排查

- 看不到浏览器窗口：默认是 headless；设置 `REXOS_BROWSER_HEADLESS=0`（或在 `browser_navigate` 传 `headless=false`）。
