# Long Task With Harness

The harness is for tasks that won’t fit in a single model context window. It makes progress **durable** by combining:

- A workspace directory with durable artifacts (`features.json`, `rexos-progress.md`, init scripts)
- A verification script (`init.sh` on Unix, `init.ps1` on Windows)
- Git commits as checkpoints
- A session id that persists per-workspace

## 1) Create a workspace

```bash
mkdir -p /tmp/rexos-task
```

## 2) Initialize the harness

Without a prompt, this only creates the durable artifacts + the initial git commit:

```bash
rexos harness init /tmp/rexos-task
```

With a prompt, RexOS runs an “initializer agent” to populate `features.json` and adjust the init script:

```bash
rexos harness init /tmp/rexos-task --prompt "Create a small CLI in this workspace that prints Hello and has a passing test suite"
```

## 3) Run an incremental session

```bash
rexos harness run /tmp/rexos-task --prompt "Implement the next failing feature"
```

The harness will:

1. Run `preflight` (show recent commits, progress tail, next failing feature)
2. Run the agent for this session
3. Run the workspace init script
4. If it fails, feed the failure back and retry (up to `--max-attempts`)
5. If it passes, checkpoint commit any changes

## 4) Repeat until done

```bash
rexos harness run /tmp/rexos-task --prompt "Continue"
```

## Where state lives

- Workspace: your code + `features.json` + `rexos-progress.md` + init scripts + git history
- Memory: `~/.rexos/rexos.db` (sessions/messages + small KV store)

