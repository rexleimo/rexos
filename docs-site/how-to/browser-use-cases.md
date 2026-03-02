# Browser Use Cases (Recipes)

This page is intentionally **copy-paste friendly**: concrete prompts, expected artifacts, and small troubleshooting notes.

See also: [Browser Automation (Playwright)](browser-automation.md).

## Prerequisites (Playwright)

Install Playwright (Python):

```bash
python3 -m pip install playwright
python3 -m playwright install chromium
```

If your Python executable isn't `python3`, set `REXOS_BROWSER_PYTHON` (example: `python`).

## 1) GUI smoke check (example.com)

**Goal:** verify `browser_*` works end-to-end and leaves evidence in your workspace.

=== "Bash (macOS/Linux)"
    ```bash
    mkdir -p rexos-demo && cd rexos-demo
    export REXOS_BROWSER_HEADLESS=0

    rexos agent run --workspace . --prompt "Use browser tools to open https://example.com, read the page, write a 3-bullet summary to notes/example.md, save a screenshot to .rexos/browser/example.png, then close the browser."
    ```

=== "PowerShell (Windows)"
    ```powershell
    mkdir rexos-demo -Force | Out-Null
    cd rexos-demo
    $env:REXOS_BROWSER_HEADLESS = "0"

    rexos agent run --workspace . --prompt "Use browser tools to open https://example.com, read the page, write a 3-bullet summary to notes/example.md, save a screenshot to .rexos/browser/example.png, then close the browser."
    ```

**What to expect**

- `notes/example.md`
- `.rexos/browser/example.png`

## 2) Real-world flow: Baidu “today’s weather” (Browser + Ollama)

**Goal:** open a Baidu results page, extract today’s weather info, and save evidence.

### Recommended model (Ollama)

Make sure Ollama has a strong instruction model (example):

```bash
ollama pull qwen3:4b
```

Then set it as default in `~/.rexos/config.toml`:

```toml
[providers.ollama]
default_model = "qwen3:4b"
```

### Run (GUI mode)

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

**What to expect**

- `notes/weather.md`
- `.rexos/browser/baidu_weather.png`

!!! note "If you hit a CAPTCHA"
    Some sites may show CAPTCHAs or block automation. If that happens, try a different query/site, or switch to `web_search` + `web_fetch` when the content is not JS-heavy.

## 3) Wikipedia: open → summarize → screenshot

**Goal:** a stable no-login site for quick demos.

=== "Bash (macOS/Linux)"
    ```bash
    export REXOS_BROWSER_HEADLESS=0

    rexos agent run --workspace . --prompt "Use browser tools to open https://en.wikipedia.org/wiki/Rust_(programming_language) . Read the page. Write a short summary to notes/wiki_rust.md. Save a screenshot to .rexos/browser/wiki_rust.png. Close the browser."
    ```

=== "PowerShell (Windows)"
    ```powershell
    $env:REXOS_BROWSER_HEADLESS = "0"

    rexos agent run --workspace . --prompt "Use browser tools to open https://en.wikipedia.org/wiki/Rust_(programming_language) . Read the page. Write a short summary to notes/wiki_rust.md. Save a screenshot to .rexos/browser/wiki_rust.png. Close the browser."
    ```

**What to expect**

- `notes/wiki_rust.md`
- `.rexos/browser/wiki_rust.png`

## 4) (From source) Run the browser + Ollama smoke test

If you're hacking on RexOS itself, you can run the ignored smoke test:

```bash
REXOS_OLLAMA_MODEL=qwen3:4b cargo test -p rexos --test browser_baidu_weather_smoke -- --ignored --nocapture
```

Expected output includes a line like:

- `[rexos][baidu_weather] summary=...`

This test uses a temp workspace and cleans it up. Use the recipes above if you want to keep screenshots and files.

## Tips

- For search engines, consider opening a **results URL** directly (more reliable than typing into the homepage search box).
- Always `browser_close` at the end (even on errors).
- Do not enter credentials or complete purchases without explicit user confirmation.
