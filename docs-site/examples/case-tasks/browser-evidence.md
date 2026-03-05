# Browser Evidence Capture (Browser + Ollama)

**Goal:** open a real page, extract key facts, and save a screenshot as evidence.

This is a good first “browser + LLM” workflow.

## Prereqs

- Ollama running locally (`ollama serve`)
- A Chromium-based browser installed (Chrome/Chromium/Edge)

## Run (GUI mode)

=== "Bash (macOS/Linux)"
    ```bash
    export REXOS_BROWSER_HEADLESS=0

    loopforge agent run --workspace . --prompt "Use browser tools to open https://en.wikipedia.org/wiki/Large_language_model . Wait for #firstHeading (browser_wait). Run browser_run_js to extract document.title and the heading text. Scroll down by 800px (browser_scroll). Read the page and write notes/browser_evidence.md with: URL, title, heading, and a 5-bullet summary based only on the page text. Save a screenshot to .rexos/browser/wikipedia_llm.png. Close the browser."
    ```

=== "PowerShell (Windows)"
    ```powershell
    $env:REXOS_BROWSER_HEADLESS = "0"

    loopforge agent run --workspace . --prompt "Use browser tools to open https://en.wikipedia.org/wiki/Large_language_model . Wait for #firstHeading (browser_wait). Run browser_run_js to extract document.title and the heading text. Scroll down by 800px (browser_scroll). Read the page and write notes/browser_evidence.md with: URL, title, heading, and a 5-bullet summary based only on the page text. Save a screenshot to .rexos/browser/wikipedia_llm.png. Close the browser."
    ```

## What to expect

- `notes/browser_evidence.md`
- `.rexos/browser/wikipedia_llm.png`

!!! note "If the browser doesn't appear"
    Browser tools are headless by default. Make sure `REXOS_BROWSER_HEADLESS=0` (or pass `headless=false` on the first `browser_navigate` call).

