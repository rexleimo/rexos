# 新仓库上手（10 分钟）

**目标：** 为刚 clone 的代码库生成一份快速上手笔记（`notes/onboarding.md`）。

## 运行

1) 先 `cd` 到你想理解的仓库目录。

2) 执行：

=== "macOS/Linux"
    ```bash
    loopforge agent run --workspace . --prompt "请帮我快速上手这个仓库：优先使用 file_list（必要时才用 shell）了解顶层结构；如果有 README 就读一下；通过检查 Cargo.toml/package.json/pyproject.toml/go.mod 等文件推断 build/test 命令。最后写出 notes/onboarding.md，包含：1) 这个仓库是做什么的 2) 如何构建 3) 如何测试 4) 关键目录（要点） 5) 下一步行动（3 条）。尽量简洁实用。不要安装依赖，不要跑很重的命令。"
    ```

=== "Windows (PowerShell)"
    ```powershell
    loopforge agent run --workspace . --prompt "请帮我快速上手这个仓库：优先使用 file_list（必要时才用 shell）了解顶层结构；如果有 README 就读一下；通过检查 Cargo.toml/package.json/pyproject.toml/go.mod 等文件推断 build/test 命令。最后写出 notes/onboarding.md，包含：1) 这个仓库是做什么的 2) 如何构建 3) 如何测试 4) 关键目录（要点） 5) 下一步行动（3 条）。尽量简洁实用。不要安装依赖，不要跑很重的命令。"
    ```

## 预期产物

- `notes/onboarding.md`

!!! tip "更强一点（可选）"
    如果你希望它顺手验证一下，可以在 prompt 里加一句：“如果 tests 看起来很快，就跑一下（超过 60 秒就停止）。”

