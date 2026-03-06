# 上手排障

当 `loopforge onboard` 或 `loopforge doctor` 给出 warning/error 时，先看这页。

## 1) 配置无效

现象：

- `config invalid: ~/.loopforge/config.toml`
- `loopforge doctor` 里的 `config.parse` 报错

处理方式：

1. 运行 `loopforge config validate`
2. 修正 TOML 语法或 provider/router 名称
3. 重跑 `loopforge doctor`

## 2) Provider 不可达

现象：

- `ollama.http` warning/error
- onboarding 因 timeout / connection refused / HTTP 错误失败

处理方式：

```bash
ollama serve
ollama list
loopforge doctor
```

如果你不是用 Ollama，请检查 provider base URL 和凭证是否正确。

## 3) 模型不可用

现象：

- onboarding 提示 “model not found”
- provider 可达，但首任务起不来

处理方式：

```bash
ollama list
```

然后：

- 拉取一个本地对话模型，或
- 修改 `~/.loopforge/config.toml` 中 `[providers.ollama].default_model`

## 4) 浏览器前置条件缺失

现象：

- `browser.chromium` warning
- 浏览器类用例在打开页面前就失败

处理方式：

- 安装 Chromium 内核浏览器，或
- 设置 `LOOPFORGE_BROWSER_CHROME_PATH`，或
- 设置 `LOOPFORGE_BROWSER_CDP_HTTP` 指向可用的 Chromium DevTools 端点

## 5) 先看哪个文件？

先打开最近一次 workspace 报告：

- `<workspace>/.loopforge/onboard-report.md`

它会最快告诉你：

- 哪些通过了
- 哪些失败了
- 推荐下一条命令是什么
- 下一批 starter tasks 是什么
