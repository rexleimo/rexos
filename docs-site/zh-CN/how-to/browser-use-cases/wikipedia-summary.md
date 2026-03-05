# Wikipedia：总结 + 截图

**目标：** 一个更稳定、无需登录的网站，用于快速演示。

另见：[浏览器自动化（CDP）](../browser-automation.md)。

## 运行（有界面模式）

=== "Bash (macOS/Linux)"
    ```bash
    export REXOS_BROWSER_HEADLESS=0

    loopforge agent run --workspace . --prompt "使用 browser 工具打开 https://en.wikipedia.org/wiki/Rust_(programming_language) 。读取页面内容，把简短总结写到 notes/wiki_rust.md，并把截图保存到 .rexos/browser/wiki_rust.png，最后关闭浏览器。"
    ```

=== "PowerShell (Windows)"
    ```powershell
    $env:REXOS_BROWSER_HEADLESS = "0"

    loopforge agent run --workspace . --prompt "使用 browser 工具打开 https://en.wikipedia.org/wiki/Rust_(programming_language) 。读取页面内容，把简短总结写到 notes/wiki_rust.md，并把截图保存到 .rexos/browser/wiki_rust.png，最后关闭浏览器。"
    ```

## 预期结果

- `notes/wiki_rust.md`
- `.rexos/browser/wiki_rust.png`
