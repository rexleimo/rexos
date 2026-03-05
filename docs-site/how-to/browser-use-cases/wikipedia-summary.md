# Wikipedia Summary (with Screenshot)

**Goal:** a stable no-login site for quick demos.

See also: [Browser Automation (CDP)](../browser-automation.md).

## Run (GUI mode)

=== "Bash (macOS/Linux)"
    ```bash
    export REXOS_BROWSER_HEADLESS=0

    loopforge agent run --workspace . --prompt "Use browser tools to open https://en.wikipedia.org/wiki/Rust_(programming_language) . Read the page. Write a short summary to notes/wiki_rust.md. Save a screenshot to .rexos/browser/wiki_rust.png. Close the browser."
    ```

=== "PowerShell (Windows)"
    ```powershell
    $env:REXOS_BROWSER_HEADLESS = "0"

    loopforge agent run --workspace . --prompt "Use browser tools to open https://en.wikipedia.org/wiki/Rust_(programming_language) . Read the page. Write a short summary to notes/wiki_rust.md. Save a screenshot to .rexos/browser/wiki_rust.png. Close the browser."
    ```

## What to expect

- `notes/wiki_rust.md`
- `.rexos/browser/wiki_rust.png`
