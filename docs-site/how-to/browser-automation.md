# Browser Automation (Playwright)

Use browser tools when `web_fetch` isn't enough (JS-rendered pages, multi-step flows, forms, screenshots).

## Prerequisites

Install Playwright (Python):

```bash
python3 -m pip install playwright
python3 -m playwright install chromium
```

If your Python executable isn't `python3`, set `REXOS_BROWSER_PYTHON` (example: `python`).

## Tool set

- `browser_navigate` — open a URL (SSRF-protected by default)
- `browser_click` — click by CSS selector (best-effort text fallback)
- `browser_type` — fill an input
- `browser_read_page` — return `{title,url,content}` (content is truncated)
- `browser_screenshot` — write a PNG to a workspace-relative path
- `browser_close` — close the session (idempotent)

## Recommended loop

1. `browser_navigate` to the entry page
2. `browser_read_page` to confirm state
3. One small action: `browser_click` or `browser_type`
4. `browser_read_page` again to confirm the page changed
5. Repeat until done, then `browser_screenshot` for evidence and `browser_close`

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
You may use RexOS browser tools (browser_navigate/click/type/read_page/screenshot/close).

Rules:
- Always call browser_read_page after navigate/click/type to verify page state before the next step.
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
- `browser_screenshot` only writes to workspace-relative paths (no absolute paths, no `..`, no symlink escapes).

## Troubleshooting

- Error mentions Playwright missing: run the install commands in “Prerequisites”.
- Error mentions `python3` missing: set `REXOS_BROWSER_PYTHON=python`.
- Error mentions session not started: call `browser_navigate` first.
