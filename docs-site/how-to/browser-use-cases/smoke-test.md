# Smoke Test (From Source)

If you're hacking on LoopForge itself, run smoke checks in two layers:

1. Stable baseline (Wikipedia + Ollama): validates browser + LLM pipeline with low page volatility.
2. Real-world scenario (Baidu weather + Ollama): validates a dynamic Chinese search workflow.

```bash
# 1) Stable baseline smoke (recommended first)
REXOS_OLLAMA_MODEL=qwen3:4b cargo test -p rexos --test browser_wikipedia_smoke -- --ignored --nocapture

# 2) Real-world scenario smoke (dynamic site, may vary)
REXOS_OLLAMA_MODEL=qwen3:4b cargo test -p rexos --test browser_baidu_weather_smoke -- --ignored --nocapture
```

Notes:

- The default browser backend is **CDP**, so you need a local Chromium-based browser (Chrome/Chromium/Edge).
- If you want to force the legacy Playwright backend for this smoke test:
  - `export REXOS_BROWSER_BACKEND=playwright`
  - then install Playwright as described in [Browser Automation](../browser-automation.md).
- By default these tests use a temp workspace and clean it up. If you want to keep screenshots + page dumps:
  - `export REXOS_BROWSER_SMOKE_WORKSPACE=./rexos-browser-smoke` (or any directory)

Expected output includes a line like:

- `[rexos][wikipedia_smoke] summary=...`
- `[rexos][baidu_weather] summary=...`

When `REXOS_BROWSER_SMOKE_WORKSPACE` is set, the test writes:

- `.rexos/browser/wikipedia_home.png`
- `notes/wikipedia_summary.md`
- `.rexos/browser/baidu_weather.png`
- `notes/baidu_weather_page.txt`
- `notes/weather.md`
