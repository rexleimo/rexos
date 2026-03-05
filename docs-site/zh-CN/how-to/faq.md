# 新手 FAQ

这页是给新手准备的“短答案 + 可复制命令”。
如果你卡住，先跑一遍 `loopforge doctor`（`rexos doctor` 仍兼容）。

## 1）刚安装完，最小可运行流程是什么？

```bash
ollama serve
loopforge init
mkdir -p rexos-demo
loopforge agent run --workspace rexos-demo --prompt "Create hello.txt with the word hi"
cat rexos-demo/hello.txt
```

预期：`hello.txt` 存在，内容是 `hi`。

## 2）Ollama 首选什么模型？

优先用你本机已有的小型对话模型（例如 `qwen3:4b`）。

```bash
ollama list
```

没有就先拉：

```bash
ollama pull qwen3:4b
```

然后在 `~/.rexos/config.toml` 设置：

```toml
[providers.ollama]
default_model = "qwen3:4b"
```

## 3）怎么快速判断环境健康？

```bash
loopforge config validate
loopforge doctor
```

`doctor` 里至少应看到：
- config 解析成功
- provider 连通性正常
- 浏览器/CDP 可用（如果你要跑浏览器工具）

## 4）为什么 `agent run` 看起来“卡住了”？

常见原因：
- 模型太弱，工具调用链复杂
- prompt 目标过大
- 目标网站是动态页面/反爬页面

建议：
- 先把任务范围缩小
- 换更强模型
- 在 prompt 里加明确输出与验证标准

示例：

```bash
loopforge agent run --workspace rexos-demo --prompt "Read README.md and write notes/summary.md with 5 bullets."
```

## 5）为什么会报“tool arguments/JSON 参数错误”？

通常是模型生成的工具参数格式漂移（参数 JSON 不合法）。

实用处理方式：
- 简化 prompt
- 在 prompt 里明确参数 key
- 工具密集任务切到更强模型/provider

## 6）在真实仓库里如何安全使用？

建议先用独立 workspace：

```bash
mkdir -p /tmp/rexos-work
loopforge agent run --workspace /tmp/rexos-work --prompt "..."
```

仓库改动建议走 harness：

```bash
loopforge harness init my-repo
loopforge harness run my-repo --prompt "Run tests and fix one failing case"
```

## 7）浏览器任务在某些网站失败，是不是 LoopForge 坏了？

不一定。部分网站存在反爬与动态渲染，页面结构会波动。

建议流程：
1. 先用简单站点验证浏览器链路；
2. 保留截图和页面文本作为证据；
3. 做稳定性验证时优先公开静态页面（如 Wikipedia）。

## 8）新手该怎么写 prompt？

用这个模板：

```text
Goal:
Input:
Output file:
Constraints:
Verification command:
```

示例：

```bash
loopforge agent run --workspace rexos-demo --prompt "Goal: summarize Cargo.toml dependencies. Output file: notes/deps.md. Constraints: 8 bullets max. Verification: file must exist."
```

## 9）提 bug 时需要哪些信息？

至少提供：
- 完整命令
- 最小 prompt
- model/provider
- 终端错误信息
- 可复现的 workspace 文件

建议附带：

```bash
loopforge doctor > doctor.txt
```

## 10）下一步学什么？

- 新人复习：`tutorials/new-user-walkthrough.md`
- 案例任务：`examples/case-tasks/index.md`
- 10 个可复制任务：`examples/case-tasks/ten-copy-paste-tasks.md`
- 浏览器案例：`how-to/browser-use-cases.md`
