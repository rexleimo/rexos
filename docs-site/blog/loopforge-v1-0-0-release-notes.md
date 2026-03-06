# LoopForge v1.0.0 Release Notes

LoopForge `v1.0.0` is the first hard cutover release under the LoopForge name.

## What changed

- Public branding is now consistently `LoopForge`.
- The default config/data directory is now `~/.loopforge`.
- Workspace runtime artifacts now live under `.loopforge/`.
- Public environment variables now use the `LOOPFORGE_*` prefix.
- Harness progress artifacts now use `loopforge-progress.md`.
- Public docs, examples, and repository links now point to `https://github.com/rexleimo/LoopForge`.

## Why this matters

This release removes the remaining outward-facing `RexOS` naming so new users don't land on stale commands, paths, or repository URLs.

## Upgrade notes

If you have old local docs, scripts, or shell snippets, update them to:

- CLI: `loopforge`
- Config path: `~/.loopforge/config.toml`
- Workspace artifacts: `.loopforge/`
- Env vars: `LOOPFORGE_*`

## Related links

- [Changelog](https://github.com/rexleimo/LoopForge/blob/main/CHANGELOG.md#100---2026-03-06)
- [What Is LoopForge?](what-is-loopforge.md)
- [Quickstart (Ollama)](../tutorials/quickstart-ollama.md)
