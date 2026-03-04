# GUI Smoke Check (example.com)

**Goal:** verify `browser_*` works end-to-end and leaves evidence in your workspace.

See also: [Browser Automation (CDP)](../browser-automation.md).

## Run

=== "Bash (macOS/Linux)"
    ```bash
    mkdir -p rexos-demo && cd rexos-demo
    export REXOS_BROWSER_HEADLESS=0

    rexos agent run --workspace . --prompt "Use browser tools to open https://example.com, read the page, write a 3-bullet summary to notes/example.md, save a screenshot to .rexos/browser/example.png, then close the browser."
    ```

=== "PowerShell (Windows)"
    ```powershell
    mkdir rexos-demo -Force | Out-Null
    cd rexos-demo
    $env:REXOS_BROWSER_HEADLESS = "0"

    rexos agent run --workspace . --prompt "Use browser tools to open https://example.com, read the page, write a 3-bullet summary to notes/example.md, save a screenshot to .rexos/browser/example.png, then close the browser."
    ```

## Option B: Docker GUI sandbox (Chromium + noVNC)

If you don’t want to install Chrome/Chromium locally, run a sandboxed GUI Chromium in Docker and attach via CDP.

1) Start the sandbox container (from the RexOS repo root):

```bash
scripts/browser_sandbox_up.sh up --build
```

2) Open the noVNC observer UI:

- URL: `http://127.0.0.1:6080/vnc.html`
- Password: `rexos` (from `docker/sandbox-browser/compose.yml`)

3) In another terminal, point RexOS browser tools at the sandbox CDP:

=== "Bash (macOS/Linux)"
    ```bash
    export REXOS_BROWSER_CDP_HTTP="http://127.0.0.1:9222"
    ```

=== "PowerShell (Windows)"
    ```powershell
    $env:REXOS_BROWSER_CDP_HTTP = "http://127.0.0.1:9222"
    ```

Now run the same GUI smoke check prompt as above.

## What to expect

- `notes/example.md`
- `.rexos/browser/example.png`

## Troubleshooting

- No browser window appears: it’s headless by default; set `REXOS_BROWSER_HEADLESS=0` (or pass `headless=false` to `browser_navigate`).
