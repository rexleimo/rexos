# 常见场景（配方）

这个页面尽量写成“可复制粘贴”的配方：命令、预期产物、下一步怎么做。

## 先选对模式

- `rexos agent run`：一次性任务（在 workspace 沙盒里执行工具调用，你 review/commit 结果）
- `rexos harness init/run`：长任务（**验证 + checkpoint**；适合“持续迭代直到 X 通过”）
- `rexos daemon start`：最小化 HTTP daemon（目前只有 `/healthz`），用于集成/健康检查

---

## 1) 用 Harness 把“修到测试通过”变成可持续推进

**目标：** 让 agent 按 “修改 → 验证 → checkpoint” 循环持续推进，失败可回滚。

### 步骤

1) 在你要修改的 repo 里初始化 harness（推荐直接在 repo 根目录）：

```bash
cd /path/to/your/repo
rexos harness init . --prompt "创建一个 checklist：测试全部通过、lint 干净、基础 smoke check"
```

2) 按项目需求改 verifier 脚本（测试/构建/lint 等）：

=== "Bash (macOS/Linux)"
    ```bash
    ./init.sh
    ```

=== "PowerShell (Windows)"
    ```powershell
    .\init.ps1
    ```

3) 反复跑增量循环直到 verifier 通过：

```bash
rexos harness run . --prompt "继续。优先处理 verifier 输出里最先失败的部分。"
```

### 你会看到什么

- workspace 会有持久化产物：
  - `features.json`（checklist）
  - `rexos-progress.md`（只追加的进度日志）
  - `init.sh` + `init.ps1`（你的 verifier 脚本）
- 当 verifier 通过时，RexOS 会创建 **checkpoint git commit**。

!!! tip "回滚方式与普通 git 一样"
    例如 `git reset --hard HEAD~1` 回退到上一个 checkpoint，然后继续 `rexos harness run`。

---

## 2) 多文件机械化改动（用 workspace 沙盒保护边界）

适合“改一堆文件，但希望你自己 review 后再 commit”的场景：

```bash
cd /path/to/repo
rexos agent run --workspace . --prompt "把 Foo 重命名成 Bar，更新 imports，并保持测试通过。"
```

一些效果不错的 prompt：

- “全仓库替换这个 API，并跑格式化工具。”
- “更新 deprecated 调用，并补一个最小回归测试。”
- “迁移配置格式，保持兼容。”

---

## 3) 本地 Ollama 做 planning，云端模型做 coding

常见路由策略：

- planning：本地/小模型（便宜、快）
- coding：更强的云端模型
- summary：便宜的总结模型

示例（只展示 router）：

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

---

## 4) 长重构：每次 run 都尽量缩小范围

不要一次做“超大重构”，更推荐多次 harness run，每次只做一小步：

1) 拆分模块
2) 更新 imports
3) 修编译
4) 修测试
5) 跑 verifier 脚本

这样 diff 好 review，失败也好定位。

---

## 5) 让任务可复现、可分享

把 harness 产物（`features.json`、`rexos-progress.md`、init 脚本）一起提交，别人就能复用同一套长任务循环（甚至继续推进）。

---

## 6) Daemon（实验性）：用于健康检查

目前 daemon 仅提供健康检查接口：

```bash
rexos daemon start --addr 127.0.0.1:8787
curl http://127.0.0.1:8787/healthz
```

可以用于容器 readiness / supervisor；更复杂的能力建议先用 CLI。

---

## 7) 本地小模型先跑通（推荐）

先用 Ollama 小模型把工具调用 + harness 流程跑通、稳定下来，再把路由切到更强的云端模型跑大任务。

---

## 8) 浏览器自动化（Playwright bridge）

当你需要与动态网页交互（JS 渲染内容、点击、输入、截图）时，使用 `browser_*` 工具会更可靠。

另见：[浏览器自动化（Playwright）](browser-automation.md)。

### 前置条件

安装 Playwright（Python）：

```bash
python3 -m pip install playwright
python3 -m playwright install chromium
```

### 示例：打开页面→读取→写总结→保存截图

```bash
rexos agent run --workspace . --prompt "使用 browser 工具打开 https://example.com，读取页面内容，把简短总结写到 notes/example.md，并把截图保存到 .rexos/browser/example.png。"
```

注意：

- `browser_navigate` 默认带 SSRF 防护（只有本地/私网目标才建议显式开启 `allow_private=true`）。
- 截图只允许写到 workspace 相对路径（不允许绝对路径、不允许 `..`、不允许通过 symlink 逃逸）。
