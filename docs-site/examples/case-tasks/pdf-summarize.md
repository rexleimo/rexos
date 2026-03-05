# PDF Summary (pdf)

**Goal:** extract text from a PDF in your workspace and write a concise summary.

## Prereqs

- A PDF file in your workspace (or download one with the commands below).
- A configured LLM provider (Ollama is a good default for local testing).

## Get a sample PDF (optional)

=== "Bash (macOS/Linux)"
    ```bash
    mkdir -p samples
    curl -L -o samples/dummy.pdf https://www.w3.org/WAI/ER/tests/xhtml/testfiles/resources/pdf/dummy.pdf
    ```

=== "PowerShell (Windows)"
    ```powershell
    New-Item -ItemType Directory -Force samples | Out-Null
    Invoke-WebRequest -Uri "https://www.w3.org/WAI/ER/tests/xhtml/testfiles/resources/pdf/dummy.pdf" -OutFile "samples/dummy.pdf"
    ```

## Run

=== "Bash (macOS/Linux)"
    ```bash
    loopforge agent run --workspace . --prompt "Use the pdf tool to extract text from samples/dummy.pdf (max_pages=10, max_chars=12000). Then write notes/pdf_summary.md with: (1) a 6-bullet summary, (2) key terms, and (3) any missing/garbled parts you notice. Only use the extracted PDF text; do not invent content."
    ```

=== "PowerShell (Windows)"
    ```powershell
    loopforge agent run --workspace . --prompt "Use the pdf tool to extract text from samples/dummy.pdf (max_pages=10, max_chars=12000). Then write notes/pdf_summary.md with: (1) a 6-bullet summary, (2) key terms, and (3) any missing/garbled parts you notice. Only use the extracted PDF text; do not invent content."
    ```

## Optional: select pages

If you only need a subset, pass a 1-indexed page selector like `pages="2-3"` (or `pages="2,4-6"`).

## What to expect

- `notes/pdf_summary.md`
