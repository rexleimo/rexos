# Harness 配方（Checkpoints）

当你希望 RexOS “持续迭代直到 verifier 通过”，并且希望失败可回滚、过程可 checkpoint 时，用 Harness 最合适。

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

## 2) 长重构：每次 run 都尽量缩小范围

不要一次做“超大重构”，更推荐多次 harness run，每次只做一小步：

1) 拆分模块
2) 更新 imports
3) 修编译
4) 修测试
5) 跑 verifier 脚本

这样 diff 好 review，失败也好定位。

## 3) 让任务可复现、可分享

把 harness 产物（`features.json`、`rexos-progress.md`、init 脚本）一起提交，别人就能复用同一套长任务循环（甚至继续推进）。
