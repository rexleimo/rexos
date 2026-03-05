# 百度“今天天气”（Browser + Ollama）

**目标：** 打开百度搜索结果页，提取“今天天气”关键信息，并截图留证。

另见：[浏览器自动化（CDP）](../browser-automation.md)。

## 推荐模型（Ollama）

确保 Ollama 有一个比较强的指令模型（示例）：

```bash
ollama pull qwen3:4b
```

然后在 `~/.rexos/config.toml` 里设置默认模型：

```toml
[providers.ollama]
default_model = "qwen3:4b"
```

## 运行（有界面模式）

=== "Bash (macOS/Linux)"
    ```bash
    export REXOS_BROWSER_HEADLESS=0

    loopforge agent run --workspace . --prompt "使用 browser 工具打开 https://www.baidu.com/s?wd=%E5%8C%97%E4%BA%AC%20%E4%BB%8A%E5%A4%A9%E5%A4%A9%E6%B0%94 。等待 #content_left 出现后读取页面。请从页面文本中提取“今天天气”的关键信息（天气现象、温度范围、风力/风向），写入 notes/weather.md。把截图保存到 .rexos/browser/baidu_weather.png。最后关闭浏览器。如果找不到天气信息，请说明找不到，但仍要保存截图。"
    ```

=== "PowerShell (Windows)"
    ```powershell
    $env:REXOS_BROWSER_HEADLESS = "0"

    loopforge agent run --workspace . --prompt "使用 browser 工具打开 https://www.baidu.com/s?wd=%E5%8C%97%E4%BA%AC%20%E4%BB%8A%E5%A4%A9%E5%A4%A9%E6%B0%94 。等待 #content_left 出现后读取页面。请从页面文本中提取“今天天气”的关键信息（天气现象、温度范围、风力/风向），写入 notes/weather.md。把截图保存到 .rexos/browser/baidu_weather.png。最后关闭浏览器。如果找不到天气信息，请说明找不到，但仍要保存截图。"
    ```

## 预期结果

- `notes/weather.md`
- `.rexos/browser/baidu_weather.png`

!!! note "如果遇到验证码（CAPTCHA）"
    某些网站可能会弹验证码或限制自动化。如果遇到这种情况，可以换个站点/关键词，或者在内容不依赖 JS 的情况下改用 `web_search` + `web_fetch`。
