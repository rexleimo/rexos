# 机械化改动（Workspace 沙盒）

当你想要一次性、可控的改动（自己 review 后再 commit），用 `agent run` 最合适。

## 示例

```bash
cd /path/to/repo
loopforge agent run --workspace . --prompt "把 Foo 重命名成 Bar，更新 imports，并保持测试通过。"
```

一些效果不错的 prompt：

- “全仓库替换这个 API，并跑格式化工具。”
- “更新 deprecated 调用，并补一个最小回归测试。”
- “迁移配置格式，保持兼容。”
