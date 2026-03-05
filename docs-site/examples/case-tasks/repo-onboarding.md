# Repo Onboarding (10 minutes)

**Goal:** generate a quick onboarding note (`notes/onboarding.md`) for a codebase you just cloned.

## Run

1) `cd` into the repo you want to understand.

2) Run:

=== "macOS/Linux"
    ```bash
    loopforge agent run --workspace . --prompt "You are helping me onboard this repo. Use file_list (and shell only if needed) to understand the top-level structure. Read README files if present. Detect build/test commands by checking for Cargo.toml/package.json/pyproject.toml/go.mod, etc. Then write notes/onboarding.md with sections: 1) What this repo is 2) How to build 3) How to test 4) Key folders (bullets) 5) Next actions (3 bullets). Keep it concise and practical. Do not install dependencies. Do not run heavy commands."
    ```

=== "Windows (PowerShell)"
    ```powershell
    loopforge agent run --workspace . --prompt "You are helping me onboard this repo. Use file_list (and shell only if needed) to understand the top-level structure. Read README files if present. Detect build/test commands by checking for Cargo.toml/package.json/pyproject.toml/go.mod, etc. Then write notes/onboarding.md with sections: 1) What this repo is 2) How to build 3) How to test 4) Key folders (bullets) 5) Next actions (3 bullets). Keep it concise and practical. Do not install dependencies. Do not run heavy commands."
    ```

## What to expect

- `notes/onboarding.md`

!!! tip "Make it more powerful"
    If you want the agent to run a quick check, add: “Run tests if they look fast (and stop if it takes more than ~60 seconds).”

