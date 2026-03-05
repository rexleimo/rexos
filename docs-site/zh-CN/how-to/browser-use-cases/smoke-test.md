# Smoke test（从源码）

如果你在开发 LoopForge 本身，建议按两层跑 smoke：

1. 稳定基线（Wikipedia + Ollama）：优先验证浏览器 + LLM 链路是否通畅。
2. 真实场景（百度天气 + Ollama）：验证动态站点场景。

```bash
# 1) 稳定基线 smoke（建议先跑）
REXOS_OLLAMA_MODEL=qwen3:4b cargo test -p rexos --test browser_wikipedia_smoke -- --ignored --nocapture

# 2) 真实场景 smoke（动态网站，结果可能波动）
REXOS_OLLAMA_MODEL=qwen3:4b cargo test -p rexos --test browser_baidu_weather_smoke -- --ignored --nocapture
```

预期输出会包含类似：

- `[rexos][wikipedia_smoke] summary=...`
- `[rexos][baidu_weather] summary=...`

注意：

- 默认浏览器后端是 **CDP**，因此你需要本机安装 Chromium 系浏览器（Chrome/Chromium/Edge）。
- 该测试默认使用临时 workspace 并会自动清理；如果你想保留截图/页面 dump：
  - `export REXOS_BROWSER_SMOKE_WORKSPACE=./rexos-browser-smoke`（或任意目录）

当你设置了 `REXOS_BROWSER_SMOKE_WORKSPACE` 后，测试会写入：

- `.rexos/browser/wikipedia_home.png`
- `notes/wikipedia_summary.md`
- `.rexos/browser/baidu_weather.png`
- `notes/baidu_weather_page.txt`
- `notes/weather.md`
