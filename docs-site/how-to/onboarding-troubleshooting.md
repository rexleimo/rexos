# Onboarding Troubleshooting

Use this page when `loopforge onboard` or `loopforge doctor` reports warnings/errors.

## 1) Config invalid

Symptoms:

- `config invalid: ~/.loopforge/config.toml`
- `config.parse` is an error in `loopforge doctor`

What to do:

1. Run `loopforge config validate`
2. Fix TOML syntax or provider/router names
3. Re-run `loopforge doctor`

## 2) Provider unreachable

Symptoms:

- `ollama.http` warning/error
- onboarding fails with timeout / connection refused / HTTP request errors

What to do:

```bash
ollama serve
ollama list
loopforge doctor
```

If you are not using Ollama, verify your configured provider base URL and credentials.

## 3) Model unavailable

Symptoms:

- onboarding fails with “model not found”
- first task cannot start even though the provider is reachable

What to do:

```bash
ollama list
```

Then either:

- pull a local chat model, or
- update `[providers.ollama].default_model` in `~/.loopforge/config.toml`

## 4) Browser prerequisites missing

Symptoms:

- `browser.chromium` warning
- browser use cases fail before they can open a page

What to do:

- install a Chromium-based browser, or
- set `LOOPFORGE_BROWSER_CHROME_PATH`, or
- point `LOOPFORGE_BROWSER_CDP_HTTP` to a live Chromium DevTools endpoint

## 5) Starter says success but the expected file is missing

Symptoms:

- `onboard-report.md` shows `expected_artifact_missing`
- built-in starters like `workspace-brief` do not create their target file

What to do:

- inspect the saved session id in `<workspace>/.loopforge/onboard-report.md`
- retry the same task with the recommended `loopforge agent run ... --session ...` command
- if the assistant only printed JSON-like tool text, upgrade to this fix and retry

## 6) What file should I inspect first?

Open the most recent workspace report:

- `<workspace>/.loopforge/onboard-report.md`

It is the fastest summary of:

- what passed
- what failed
- recommended next command
- suggested starter tasks
