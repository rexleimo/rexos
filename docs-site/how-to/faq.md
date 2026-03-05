# Beginner FAQ

This page is intentionally short and copy/paste-friendly.
If you are blocked, run `loopforge doctor` first (`rexos doctor` remains compatible).

## 1) I installed RexOS. What is the minimum first run?

```bash
ollama serve
loopforge init
mkdir -p rexos-demo
loopforge agent run --workspace rexos-demo --prompt "Create hello.txt with the word hi"
cat rexos-demo/hello.txt
```

Expected: `hello.txt` exists and contains `hi`.

## 2) Which Ollama model should I use first?

Use a small chat model you already have (for example `qwen3:4b`).

```bash
ollama list
```

If needed:

```bash
ollama pull qwen3:4b
```

Then set `~/.rexos/config.toml`:

```toml
[providers.ollama]
default_model = "qwen3:4b"
```

## 3) How do I know config and dependencies are healthy?

```bash
loopforge config validate
loopforge doctor
```

`doctor` should report:
- config parsed
- provider connectivity
- browser/CDP availability (if browser tools are enabled)

## 4) Why does `agent run` look stuck?

Common causes:
- model is too weak for the task/tool pattern
- prompt is too broad
- browser targets are dynamic/anti-bot pages

Try:
- reduce scope of the prompt
- switch to a stronger model
- add explicit success criteria in the prompt

Example:

```bash
loopforge agent run --workspace rexos-demo --prompt "Read README.md and write notes/summary.md with 5 bullets."
```

## 5) Why did a tool call fail with argument errors?

This is usually model output format drift (invalid JSON arguments).

Practical mitigations:
- simplify prompt
- force explicit argument keys in prompt text
- retry with stronger model/provider for tool-heavy tasks

## 6) How do I safely run in a real repository?

Always use a dedicated workspace and commit early:

```bash
mkdir -p /tmp/rexos-work
loopforge agent run --workspace /tmp/rexos-work --prompt "..."
```

For repository work, prefer harness:

```bash
loopforge harness init my-repo
loopforge harness run my-repo --prompt "Run tests and fix one failing case"
```

## 7) Browser + web tasks fail on specific sites. Is that a bug?

Not always. Some websites use anti-bot or dynamic rendering.

Use this strategy:
1. Verify browser basics with a simple target first.
2. Keep artifacts (`screenshot`, page dump) for evidence.
3. Use fallback tasks (Wikipedia/public docs) when validating pipelines.

## 8) How do I write better prompts for beginners?

Use this template:

```text
Goal:
Input:
Output file:
Constraints:
Verification command:
```

Example:

```bash
loopforge agent run --workspace rexos-demo --prompt "Goal: summarize Cargo.toml dependencies. Output file: notes/deps.md. Constraints: 8 bullets max. Verification: file must exist."
```

## 9) What should I include in a bug report?

Include:
- exact command
- minimal prompt
- model/provider
- terminal error
- reproducible workspace files

Suggested capture:

```bash
loopforge doctor > doctor.txt
```

## 10) Where should I continue learning?

- New User Walkthrough: `tutorials/new-user-walkthrough.md`
- Case Tasks: `examples/case-tasks/index.md`
- 10 Copy/Paste Tasks: `examples/case-tasks/ten-copy-paste-tasks.md`
- Browser Use Cases: `how-to/browser-use-cases.md`
