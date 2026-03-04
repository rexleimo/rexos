# Browser Automation (CDP)

Use browser tools when `web_fetch` isn't enough (JS-rendered pages, multi-step flows, forms, screenshots).

For copy-paste recipes, see: [Browser Use Cases](browser-use-cases.md).

## Default backend: CDP (no Python)

RexOS launches a local Chromium-based browser (Chrome / Chromium / Edge) and drives it via **Chrome DevTools Protocol (CDP)**.

### Prerequisites

- Install a Chromium-based browser (Chrome/Chromium/Edge).
- If RexOS can’t find it, set `REXOS_BROWSER_CHROME_PATH` to the browser executable path.

### Remote CDP (optional)

If you already have a browser running with a remote debugging port (or you run one in Docker), point RexOS at it:

```bash
export REXOS_BROWSER_CDP_HTTP="http://127.0.0.1:9222"
```

By default, RexOS only allows **loopback** CDP endpoints. To attach to a non-loopback CDP URL, you must explicitly opt in:

```bash
export REXOS_BROWSER_CDP_ALLOW_REMOTE=1
```

For a copy/paste Docker GUI sandbox (Chromium + noVNC) that exposes CDP on `127.0.0.1:9222`, run:

```bash
scripts/browser_sandbox_up.sh up --build
```

Then follow: [GUI Smoke Check](browser-use-cases/gui-smoke-check.md).

## Headless vs GUI

By default, RexOS launches the browser in **headless** mode.

To show the browser window (local debugging / demos), set `headless=false` on the first `browser_navigate` call:

```json
{ "url": "https://www.baidu.com", "headless": false }
```

You can also set `REXOS_BROWSER_HEADLESS=0` to make GUI mode the default when `headless` is not provided.

## Optional backend: Playwright bridge (legacy)

If you prefer Playwright (or you’re in an environment where CDP is hard), switch the backend:

```bash
export REXOS_BROWSER_BACKEND=playwright
```

Then install Playwright (Python):

```bash
python3 -m pip install playwright
python3 -m playwright install chromium
```

If your Python executable isn't `python3`, set `REXOS_BROWSER_PYTHON` (example: `python`).

## Tool set

- `browser_navigate` — open a URL (SSRF-protected by default)
- `browser_back` — go back in history
- `browser_scroll` — scroll the page
- `browser_click` — click by CSS selector (best-effort text fallback)
- `browser_type` — fill an input
- `browser_press_key` — press a key (example: `Enter` to submit a form)
- `browser_wait` — wait for a selector (compat)
- `browser_wait_for` — wait for a selector/text to appear
- `browser_read_page` — return `{title,url,content}` (content is truncated)
- `browser_run_js` — evaluate a JS expression and return the result
- `browser_screenshot` — write a PNG to a workspace-relative path
- `browser_close` — close the session (idempotent)

## Recommended loop

1. `browser_navigate` to the entry page
2. `browser_read_page` to confirm state
3. One small action: `browser_click` or `browser_type`
   - If you need to submit a form, use `browser_press_key` with `Enter`.
4. If the page updates async, use `browser_wait_for` (selector/text) to wait for the new state
5. `browser_read_page` again to confirm the page changed
6. Repeat until done, then `browser_screenshot` for evidence and `browser_close`

## Selector tips

Prefer stable attributes over text that may change:

- `#id`
- `[name="q"]`
- `[data-testid="submit"]`
- `button[type="submit"]`

If a CSS selector fails, `browser_click` will try a **best-effort visible-text fallback**. Be specific (avoid short ambiguous words like “OK”).

## Prompt template (copy/paste)

Use this as a starting point for agent prompts:

```text
You may use RexOS browser tools (browser_navigate/back/scroll/click/type/press_key/wait/wait_for/read_page/run_js/screenshot/close).

Rules:
- Always call browser_read_page after navigate/click/type/press_key to verify page state before the next step.
- If the page updates async, use browser_wait_for (selector/text) before browser_read_page.
- Keep actions minimal and reversible. If selectors fail, read the page and adjust selectors.
- Save a screenshot at the end to .rexos/browser/<topic>.png.
- Do NOT enter credentials or complete purchases without explicit user confirmation.
```

Example run:

```bash
rexos agent run --workspace . --prompt "Use browser tools to open https://example.com, read the page, write a short summary to notes/example.md, and save a screenshot to .rexos/browser/example.png."
```

## Security notes

- `browser_navigate` denies loopback/private targets by default. Use `allow_private=true` only for local/private testing.
- `browser_read_page` and `browser_screenshot` also enforce the same loopback/private protection (unless you enabled `allow_private`).
- Browser tools only allow `http(s)` URLs (schemes like `file:`, `data:`, `chrome:` are blocked).
- `browser_screenshot` only writes to workspace-relative paths (no absolute paths, no `..`, no symlink escapes).

## Troubleshooting

- Error mentions Chrome/Chromium not found: install a browser or set `REXOS_BROWSER_CHROME_PATH`.
- Error mentions CDP sandbox issues in Docker: set `REXOS_BROWSER_NO_SANDBOX=1` (only in trusted sandbox envs).
- Error mentions Playwright missing: set `REXOS_BROWSER_BACKEND=playwright` and install Playwright.
- Error mentions `python3` missing (Playwright backend): set `REXOS_BROWSER_PYTHON=python`.
- No browser window appears: it's headless by default; use `headless=false` (or set `REXOS_BROWSER_HEADLESS=0`).
- Error mentions session not started: call `browser_navigate` first.
