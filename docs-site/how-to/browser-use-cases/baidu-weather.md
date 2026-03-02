# Baidu Weather (Browser + Ollama)

**Goal:** open a Baidu results page, extract today’s weather info, and save evidence.

See also: [Browser Automation (CDP)](../browser-automation.md).

## Recommended model (Ollama)

Make sure Ollama has a strong instruction model (example):

```bash
ollama pull qwen3:4b
```

Then set it as default in `~/.rexos/config.toml`:

```toml
[providers.ollama]
default_model = "qwen3:4b"
```

## Run (GUI mode)

=== "Bash (macOS/Linux)"
    ```bash
    export REXOS_BROWSER_HEADLESS=0

    rexos agent run --workspace . --prompt "Use browser tools to open https://www.baidu.com/s?wd=%E5%8C%97%E4%BA%AC%20%E4%BB%8A%E5%A4%A9%E5%A4%A9%E6%B0%94 . Wait for #content_left, then read the page. Extract today's weather info (conditions, temperature range, wind) from the page text. Write it to notes/weather.md. Save a screenshot to .rexos/browser/baidu_weather.png. Close the browser. If you can't find the weather, say so, but still save the screenshot."
    ```

=== "PowerShell (Windows)"
    ```powershell
    $env:REXOS_BROWSER_HEADLESS = "0"

    rexos agent run --workspace . --prompt "Use browser tools to open https://www.baidu.com/s?wd=%E5%8C%97%E4%BA%AC%20%E4%BB%8A%E5%A4%A9%E5%A4%A9%E6%B0%94 . Wait for #content_left, then read the page. Extract today's weather info (conditions, temperature range, wind) from the page text. Write it to notes/weather.md. Save a screenshot to .rexos/browser/baidu_weather.png. Close the browser. If you can't find the weather, say so, but still save the screenshot."
    ```

## What to expect

- `notes/weather.md`
- `.rexos/browser/baidu_weather.png`

!!! note "If you hit a CAPTCHA"
    Some sites may show CAPTCHAs or block automation. If that happens, try a different query/site, or switch to `web_search` + `web_fetch` when the content is not JS-heavy.
