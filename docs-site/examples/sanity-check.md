# Setup Sanity Check

**Goal:** verify your local config, model routing, and tool sandbox in ~2 minutes.

## Steps

1) (Recommended) Run diagnostics:

```bash
loopforge doctor
```

2) Initialize once:

```bash
loopforge init
```

3) Create a scratch workspace:

```bash
mkdir -p rexos-demo
cd rexos-demo
```

4) Run a tiny task that writes files + runs a shell command:

```bash
loopforge agent run --workspace . --prompt "Create notes/hello.md with a short greeting. Then run shell command 'pwd && ls -la'. Save the command output to notes/env.txt. End by replying with the paths you wrote."
```

## What to expect

- `notes/hello.md`
- `notes/env.txt`
