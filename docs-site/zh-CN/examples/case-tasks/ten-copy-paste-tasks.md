# 10 个可复制任务（新手友好）

下面都是可直接运行的 prompt 模板。
把 `my-work` 改成你的 workspace 目录即可。

## 0）一次性准备

```bash
ollama serve
rexos init
mkdir -p my-work
```

## 1）创建文件

```bash
rexos agent run --workspace my-work --prompt "Create notes/hello.md with exactly: Hello RexOS"
```

## 2）10 分钟仓库上手

```bash
rexos agent run --workspace . --prompt "You are helping me onboard this repo. Read top-level files, detect build/test commands, then write notes/onboarding.md."
```

## 3）总结 Cargo 依赖

```bash
rexos agent run --workspace . --prompt "Read Cargo.toml and write notes/deps.md with: dependency name, why it might exist, and risk notes."
```

## 4）生成测试计划

```bash
rexos agent run --workspace . --prompt "Inspect tests and write notes/test-plan.md with smoke, integration, and failure-injection cases."
```

## 5）起草发布说明

```bash
rexos agent run --workspace . --prompt "Read recent commits and CHANGELOG.md, then write notes/release-draft.md for the next release."
```

## 6）安全快速体检备忘录

```bash
rexos agent run --workspace . --prompt "Check config/docs for security-sensitive defaults and write notes/security-memo.md with findings and mitigations."
```

## 7）带来源的网页调研 Memo

```bash
rexos agent run --workspace my-work --prompt "Research: local-first AI agent frameworks. Write notes/research.md with 5 bullets and source URLs."
```

## 8）浏览器取证

```bash
rexos agent run --workspace my-work --prompt "Open https://www.wikipedia.org, extract 5 key facts, save screenshot to .rexos/browser/wiki.png, and write notes/wiki.md."
```

## 9）PDF 总结

```bash
rexos agent run --workspace my-work --prompt "Read samples/dummy.pdf with pdf tool and write notes/pdf-summary.md with key points and action items."
```

## 10）编码前重构清单

```bash
rexos agent run --workspace . --prompt "Analyze src/ and tests/, then write notes/refactor-checklist.md with safe sequencing and rollback plan."
```

## 验证清单

每个任务跑完后，先确认产物是否落盘：

```bash
find my-work -maxdepth 4 -type f | sort
```

仓库任务则检查：

```bash
find notes -maxdepth 2 -type f | sort
```

!!! tip
    如果结果不稳定，优先切到更强模型/provider 来处理工具密集任务。
