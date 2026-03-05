# 故障排查

## 第一步：跑 `loopforge doctor`

如果你不确定哪里出了问题，建议先跑：

```bash
loopforge doctor
```

它会检查：`~/.rexos` 路径、`config.toml`、provider 的环境变量、（本地时）Ollama 连通性、浏览器/CDP 连通性，以及 `git` 等基础依赖。

## 文档站点显示 “There isn't a GitHub Pages site here” / “GitHub Pages is designed to host…”

如果你访问 `https://os.rexai.top/` 看到的是 GitHub Pages 的占位/404 页面，通常说明：

- 仓库的 Pages 还没有启用（或没有选择 GitHub Actions 作为构建来源）
- `Docs` 工作流没有成功部署

**修复（维护者操作）：**

1) 打开仓库：**Settings → Pages**
2) 在 **Build and deployment** 下，把 **Source** 设为 **GitHub Actions**
3) 去 **Actions** 里重新运行 **Docs** 工作流

**验证：**

```bash
curl -I https://os.rexai.top/
```

预期：返回 `200`（至少不再是 GitHub “Site not found” 的 404 页面）。

---

## Ollama：连不上 / 模型不存在

LoopForge 默认把 Ollama 指向 `http://127.0.0.1:11434/v1`。

1) 确认 Ollama 已启动：

```bash
ollama serve
```

2) 验证 OpenAI 兼容接口：

```bash
curl -s http://127.0.0.1:11434/v1/models | head
```

3) 如果默认模型没拉取，先 pull（或改 `~/.rexos/config.toml` 的 `default_model`）：

```bash
ollama pull llama3.2
```

---

## Windows：`bash`/WSL 相关问题

如果你看到类似错误：

- `Windows Subsystem for Linux has no installed distributions`

一般是你在 Windows 上调用了 “指向 WSL 的 bash 启动器”。建议优先使用：

- `.\init.ps1`（PowerShell）

harness workspace 会同时生成 `init.sh` 和 `init.ps1`。

---

## Provider API key 没生效

LoopForge 会从 `~/.rexos/config.toml` 里 `api_key_env` 指定的环境变量读取 key。

例如（智谱 GLM 原生）：

```bash
export ZHIPUAI_API_KEY="id.secret"
```

如果 key 形如 `id.secret`，LoopForge 会自动签发短期 JWT（无需你手动生成 token）。
