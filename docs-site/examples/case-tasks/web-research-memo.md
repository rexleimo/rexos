# Web Research Memo (Copy/Paste)

**Goal:** search the web, fetch a few sources, and write a short memo with URLs.

## Run

Edit the topic in the prompt and run:

=== "macOS/Linux"
    ```bash
    loopforge agent run --workspace . --prompt "Topic: 'effective harnesses for long-running agents'. Use web_search to find 5 relevant sources. Pick the best 3 and use web_fetch to pull their content. Write notes/research.md with: (1) 5-bullet summary, (2) 3 key takeaways, (3) open questions, (4) source URLs list. Keep it under 500 words."
    ```

=== "Windows (PowerShell)"
    ```powershell
    loopforge agent run --workspace . --prompt "Topic: 'effective harnesses for long-running agents'. Use web_search to find 5 relevant sources. Pick the best 3 and use web_fetch to pull their content. Write notes/research.md with: (1) 5-bullet summary, (2) 3 key takeaways, (3) open questions, (4) source URLs list. Keep it under 500 words."
    ```

## What to expect

- `notes/research.md`

!!! note "JS-heavy sites"
    If a site is hard to fetch with `web_fetch`, switch to browser tools and take a screenshot as evidence.

