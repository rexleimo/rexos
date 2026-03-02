# 路由示例（Ollama + 云端）

常见工作流：

- planning：本地/小模型（便宜、快）
- coding：更强的云端模型
- summary：便宜的总结模型

## 路由示例

```toml
[router.planning]
provider = "ollama"
model = "default"

[router.coding]
provider = "glm_native" # 或 minimax_native / deepseek / kimi / qwen_native ...
model = "default"

[router.summary]
provider = "ollama"
model = "default"
```

完整 provider 示例见：`how-to/providers.md`（包含 GLM/MiniMax 原生 API 与 NVIDIA NIM）。

## 小建议：先用小模型把流程跑通

先用 Ollama 小模型把工具调用 + harness 流程跑通、稳定下来，再把路由切到更强的云端模型跑大任务。
