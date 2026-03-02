# 浏览器案例（配方）

这个页面尽量写成“可复制粘贴”的配方：具体 prompt、预期产物、以及一些简短的注意事项。

另见：[浏览器自动化（Playwright）](browser-automation.md)。

## 前置条件（Playwright）

安装 Playwright（Python）：

```bash
python3 -m pip install playwright
python3 -m playwright install chromium
```

如果你的 Python 可执行文件不是 `python3`，可以通过环境变量 `REXOS_BROWSER_PYTHON` 指定（例如 `python`）。

## 1) 有界面 smoke check（example.com）

**目标：** 验证 `browser_*` 端到端可用，并在 workspace 里留下证据文件。

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

**预期结果**

- `notes/example.md`
- `.rexos/browser/example.png`

## 2) 更接近真实场景：百度“今天天气”（Browser + Ollama）

**目标：** 打开百度搜索结果页，提取“今天天气”关键信息，并截图留证。

### 推荐模型（Ollama）

确保 Ollama 有一个比较强的指令模型（示例）：

```bash
ollama pull qwen3:4b
```

然后在 `~/.rexos/config.toml` 里设置默认模型：

```toml
[providers.ollama]
default_model = "qwen3:4b"
```

### 运行（有界面模式）

=== "Bash (macOS/Linux)"
    ```bash
    export REXOS_BROWSER_HEADLESS=0

    rexos agent run --workspace . --prompt "使用 browser 工具打开 https://www.baidu.com/s?wd=%E5%8C%97%E4%BA%AC%20%E4%BB%8A%E5%A4%A9%E5%A4%A9%E6%B0%94 。等待 #content_left 出现后读取页面。请从页面文本中提取“今天天气”的关键信息（天气现象、温度范围、风力/风向），写入 notes/weather.md。把截图保存到 .rexos/browser/baidu_weather.png。最后关闭浏览器。如果找不到天气信息，请说明找不到，但仍要保存截图。"
    ```

=== "PowerShell (Windows)"
    ```powershell
    $env:REXOS_BROWSER_HEADLESS = "0"

    rexos agent run --workspace . --prompt "使用 browser 工具打开 https://www.baidu.com/s?wd=%E5%8C%97%E4%BA%AC%20%E4%BB%8A%E5%A4%A9%E5%A4%A9%E6%B0%94 。等待 #content_left 出现后读取页面。请从页面文本中提取“今天天气”的关键信息（天气现象、温度范围、风力/风向），写入 notes/weather.md。把截图保存到 .rexos/browser/baidu_weather.png。最后关闭浏览器。如果找不到天气信息，请说明找不到，但仍要保存截图。"
    ```

**预期结果**

- `notes/weather.md`
- `.rexos/browser/baidu_weather.png`

!!! note "如果遇到验证码（CAPTCHA）"
    某些网站可能会弹验证码或限制自动化。如果遇到这种情况，可以换个站点/关键词，或者在内容不依赖 JS 的情况下改用 `web_search` + `web_fetch`。

## 3) Wikipedia：打开 → 总结 → 截图

**目标：** 一个更稳定、无需登录的网站，用于快速演示。

=== "Bash (macOS/Linux)"
    ```bash
    export REXOS_BROWSER_HEADLESS=0

    rexos agent run --workspace . --prompt "使用 browser 工具打开 https://en.wikipedia.org/wiki/Rust_(programming_language) 。读取页面内容，把简短总结写到 notes/wiki_rust.md，并把截图保存到 .rexos/browser/wiki_rust.png，最后关闭浏览器。"
    ```

=== "PowerShell (Windows)"
    ```powershell
    $env:REXOS_BROWSER_HEADLESS = "0"

    rexos agent run --workspace . --prompt "使用 browser 工具打开 https://en.wikipedia.org/wiki/Rust_(programming_language) 。读取页面内容，把简短总结写到 notes/wiki_rust.md，并把截图保存到 .rexos/browser/wiki_rust.png，最后关闭浏览器。"
    ```

**预期结果**

- `notes/wiki_rust.md`
- `.rexos/browser/wiki_rust.png`

## 4)（从源码）运行浏览器 + Ollama smoke test

如果你在开发 RexOS 本身，可以运行这个被 `#[ignore]` 的 smoke test：

```bash
REXOS_OLLAMA_MODEL=qwen3:4b cargo test -p rexos --test browser_baidu_weather_smoke -- --ignored --nocapture
```

预期输出会包含类似：

- `[rexos][baidu_weather] summary=...`

注意：该测试使用临时 workspace 并会自动清理；如果你想保留截图/文件，建议用上面的配方跑 `rexos agent run`。

## 小技巧

- 对搜索引擎来说，直接打开**结果页 URL** 通常更稳（比在首页输入框里打字更不容易被拦）。
- 出错时也尽量在最后调用 `browser_close`。
- 未经用户明确确认，不要输入账号密码，也不要进行任何付费/下单操作。
