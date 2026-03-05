# 浏览器取证（Browser + Ollama）

**目标：** 打开真实网页，抽取关键字段，并保存截图作为证据。

这是一个非常适合新手的 “浏览器 + 大模型” 小闭环。

## 前置条件

- 本地 Ollama 已启动（`ollama serve`）
- 本机已安装 Chromium 系浏览器（Chrome/Chromium/Edge）

## 运行（GUI 模式）

=== "Bash (macOS/Linux)"
    ```bash
    export REXOS_BROWSER_HEADLESS=0

    loopforge agent run --workspace . --prompt "使用浏览器工具打开 https://en.wikipedia.org/wiki/Large_language_model 。等待 #firstHeading 出现（browser_wait）。用 browser_run_js 抽取 document.title 与 heading 文本。向下滚动 800px（browser_scroll）。读取页面并写 notes/browser_evidence.md：包含 URL、title、heading，以及基于页面文本的 5 条要点总结。保存截图到 .rexos/browser/wikipedia_llm.png。最后 browser_close。"
    ```

=== "PowerShell (Windows)"
    ```powershell
    $env:REXOS_BROWSER_HEADLESS = "0"

    loopforge agent run --workspace . --prompt "使用浏览器工具打开 https://en.wikipedia.org/wiki/Large_language_model 。等待 #firstHeading 出现（browser_wait）。用 browser_run_js 抽取 document.title 与 heading 文本。向下滚动 800px（browser_scroll）。读取页面并写 notes/browser_evidence.md：包含 URL、title、heading，以及基于页面文本的 5 条要点总结。保存截图到 .rexos/browser/wikipedia_llm.png。最后 browser_close。"
    ```

## 预期产物

- `notes/browser_evidence.md`
- `.rexos/browser/wikipedia_llm.png`

!!! note "看不到浏览器窗口？"
    浏览器默认是 headless。请确认 `REXOS_BROWSER_HEADLESS=0`（或第一次 `browser_navigate` 传 `headless=false`）。

