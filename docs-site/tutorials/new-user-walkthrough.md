# New User Walkthrough (10 minutes)

This walkthrough is a “sanity check” you can run after installing RexOS. You’ll verify:

- your local model (Ollama) works
- tools are sandboxed to a workspace
- memory persists across runs
- harness workspaces create durable artifacts + git checkpoints

## 0) Prerequisites

- `rexos` is installed and on your `PATH`
- Ollama is running: `ollama serve`
- you have at least one **chat model** available:

```bash
ollama list
```

If the default model (`llama3.2`) is not installed, either pull it:

```bash
ollama pull llama3.2
```

…or edit `~/.rexos/config.toml` and set:

```toml
[providers.ollama]
default_model = "qwen3:4b" # example: pick a model you already have
```

## 1) Initialize RexOS

```bash
rexos init
```

Expected artifacts:

- `~/.rexos/config.toml`
- `~/.rexos/rexos.db`

## 2) Run a one-shot agent session (workspace sandbox)

=== "macOS/Linux"
    ```bash
    mkdir -p rexos-demo
    rexos agent run --workspace rexos-demo --prompt "Create hello.txt with the word hi"
    cat rexos-demo/hello.txt
    ```

=== "Windows (PowerShell)"
    ```powershell
    mkdir rexos-demo
    rexos agent run --workspace rexos-demo --prompt "Create hello.txt with the word hi"
    Get-Content .\rexos-demo\hello.txt
    ```

Expected:

- `hello.txt` exists in the workspace and contains `hi`
- RexOS prints a `session_id` to stderr and also persists it under `rexos-demo/.rexos/session_id`

## 3) Re-run in the same workspace (memory)

```bash
rexos agent run --workspace rexos-demo --prompt "Append a newline + bye to hello.txt"
```

Verify the file updated:

=== "macOS/Linux"
    ```bash
    cat rexos-demo/hello.txt
    ```

=== "Windows (PowerShell)"
    ```powershell
    Get-Content .\rexos-demo\hello.txt
    ```

## 4) Create a harness workspace (durable artifacts + git)

=== "macOS/Linux"
    ```bash
    mkdir -p rexos-harness-demo
    rexos harness init rexos-harness-demo
    ```

=== "Windows (PowerShell)"
    ```powershell
    mkdir rexos-harness-demo
    rexos harness init rexos-harness-demo
    ```

Expected files in `rexos-harness-demo/`:

- `features.json`
- `rexos-progress.md`
- `init.sh` and `init.ps1`
- a `.git/` directory with an initial commit

Run the harness preflight (no prompt):

```bash
rexos harness run rexos-harness-demo
```

## 5) Docs buttons (reproducibility)

On the docs site, every page should have:

- **Edit this page** → opens GitHub at `docs-site/...`
- **View source** → opens the raw Markdown file

If these buttons are missing or broken, check the docs workflow and `mkdocs.yml` (`repo_url` + `edit_uri`).
