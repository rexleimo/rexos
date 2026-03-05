# Troubleshooting

## First step: run `loopforge doctor`

If you’re not sure what’s wrong, start here:

```bash
loopforge doctor
```

It checks: `~/.rexos` paths, `config.toml`, provider env vars, Ollama connectivity (when local), browser/CDP connectivity, and core tooling like `git`.

## Docs site shows “There isn't a GitHub Pages site here” / “GitHub Pages is designed to host…”

If `https://os.rexai.top/` shows the GitHub Pages placeholder/404 page, it usually means the repo's Pages site hasn’t been enabled yet (or the Docs workflow never deployed).

**Fix (repo maintainer):**

1) Open the repo: **Settings → Pages**
2) Under **Build and deployment**, set **Source** to **GitHub Actions**
3) Re-run the **Docs** workflow in **Actions**

**Validate:**

```bash
curl -I https://os.rexai.top/
```

Expected: `200` (or at least not the GitHub “Site not found” 404).

---

## Ollama: connection refused / model not found

RexOS defaults to Ollama at `http://127.0.0.1:11434/v1`.

1) Make sure Ollama is running:

```bash
ollama serve
```

2) Check the OpenAI-compatible endpoint:

```bash
curl -s http://127.0.0.1:11434/v1/models | head
```

3) If your model isn’t available, pull it (or change `default_model` in `~/.rexos/config.toml`):

```bash
ollama pull llama3.2
```

---

## Windows: `bash`/WSL issues

RexOS supports Windows natively (PowerShell). If you see errors like:

- `Windows Subsystem for Linux has no installed distributions`

You’re likely invoking `bash` via a WSL shim. Prefer:

- `.\init.ps1` (PowerShell) on Windows

The harness workspace includes both `init.sh` and `init.ps1`.

---

## Provider API keys not picked up

Providers read keys from the env var in `api_key_env` in `~/.rexos/config.toml`.

Example (Zhipu GLM native):

```bash
export ZHIPUAI_API_KEY="id.secret"
```

If the key looks like `id.secret`, RexOS will sign a short-lived JWT automatically.
