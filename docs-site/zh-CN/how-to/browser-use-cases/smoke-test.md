# Smoke test（从源码）

如果你在开发 RexOS 本身，可以运行这个被 `#[ignore]` 的 smoke test：

```bash
REXOS_OLLAMA_MODEL=qwen3:4b cargo test -p rexos --test browser_baidu_weather_smoke -- --ignored --nocapture
```

预期输出会包含类似：

- `[rexos][baidu_weather] summary=...`

注意：该测试使用临时 workspace 并会自动清理；如果你想保留截图/文件，建议用其它配方跑 `rexos agent run`。
