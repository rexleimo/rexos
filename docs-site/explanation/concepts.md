# Concepts

RexOS is designed for long-running work where “one prompt” isn’t enough.

## Workspace

Most RexOS commands operate on a **workspace directory**:

- tools are sandboxed to it (filesystem + shell working directory)
- the harness stores durable artifacts there

## Memory (SQLite)

RexOS persists:

- sessions
- chat messages
- small key/value state

in `~/.rexos/rexos.db`, so later runs can resume with context.

## Tools (sandboxed)

The agent can call tools such as:

- `fs_read` / `fs_write` (workspace-only, blocks `..` traversal)
- `shell` (workspace-only)
- `web_fetch` (SSRF-protected by default)
- `browser_*` (headless browser automation via CDP; Playwright is optional)

!!! note "Browser tools prerequisites"
    By default, `browser_*` uses a local Chromium-based browser (Chrome/Chromium/Edge) via **CDP**.

    If RexOS can’t find a browser executable, set `REXOS_BROWSER_CHROME_PATH`.

    Optional legacy backend: set `REXOS_BROWSER_BACKEND=playwright` and install Python + Playwright:

    ```bash
    python3 -m pip install playwright
    python3 -m playwright install chromium
    ```

## Model routing

RexOS classifies runs into a task kind:

- planning
- coding
- summary

Each kind can route to a different provider/model pair.

## Harness (durable long tasks)

The harness adds a workflow on top:

1. initialize a workspace with durable artifacts
2. run incremental sessions
3. verify via `init.sh` / `init.ps1`
4. checkpoint via git commits
