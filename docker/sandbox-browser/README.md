# Browser Sandbox (Chromium + noVNC + CDP)

This sandbox runs Chromium in Docker and exposes:

- CDP HTTP endpoint: `http://127.0.0.1:9222`
- noVNC observer UI: `http://127.0.0.1:6080/vnc.html`

## Quick start

From repo root:

```bash
scripts/browser_sandbox_up.sh up --build
```

Then in another terminal:

```bash
export REXOS_BROWSER_CDP_HTTP="http://127.0.0.1:9222"
```

Now `rexos agent run` can use browser tools through remote CDP.

## Stop

```bash
scripts/browser_sandbox_up.sh down
```

## Security note

Loopback CDP is allowed by default. For non-loopback endpoints, RexOS requires explicit opt-in:

```bash
export REXOS_BROWSER_CDP_ALLOW_REMOTE=1
```

Only enable this in trusted network environments.
