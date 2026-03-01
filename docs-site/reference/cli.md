# CLI Reference

RexOS ships a single binary: `rexos`.

## Top-level commands

- `rexos init` — initialize `~/.rexos` (config + database)
- `rexos agent run` — run a single agent session in a workspace
- `rexos harness init` — initialize a harness workspace (durable artifacts + git)
- `rexos harness run` — run an incremental harness session
- `rexos daemon start` — start the HTTP daemon

## Examples

```bash
rexos init

rexos agent run --workspace /tmp/rexos-work --prompt "Create hello.txt"

rexos harness init /tmp/task --prompt "Initialize a features checklist for refactoring this repo"
rexos harness run /tmp/task --prompt "Continue"

rexos daemon start --addr 127.0.0.1:8787
```

