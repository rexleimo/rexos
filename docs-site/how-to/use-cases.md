# Common Use Cases

These are patterns that work well with RexOS today.

## 1) “Make progress without breaking the repo”

Use the harness: the loop is “change → run init script → checkpoint commit”.

```bash
rexos harness init /tmp/task --prompt "Improve this project incrementally until the test suite passes"
rexos harness run /tmp/task --prompt "Continue"
```

## 2) Mechanical edits across many files

Use `agent run` with a workspace sandbox and keep changes reviewable via git.

```bash
rexos agent run --workspace /path/to/repo --prompt "Rename Foo to Bar across the codebase and keep tests passing"
```

## 3) Provider experimentation without rewriting your app

Keep your logic the same; switch providers/models in `~/.rexos/config.toml`.

- Local: Ollama
- Cloud: GLM / MiniMax / Qwen native

## 4) Long refactors with checkpoints

Break work into multiple harness runs:

- isolate a module
- update imports
- fix tests
- run the init script

## 5) Reproducible “agent tasks” you can share

Commit the workspace artifacts (`features.json`, `rexos-progress.md`, init scripts) so others can reproduce the loop.

## 6) Daemon mode for integrations (experimental)

Run the daemon and poll `/healthz`:

```bash
rexos daemon start --addr 127.0.0.1:8787
curl http://127.0.0.1:8787/healthz
```

## 7) Local testing with small models

Use Ollama with small models to validate tool-calling + harness flow before scaling up.

