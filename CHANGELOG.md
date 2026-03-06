# Changelog

All notable user-visible changes are documented in this file.

## [Unreleased]

### Added

- Versioning policy documented in `docs/versioning-and-release.md`.
- Skills MVP baseline:
  - New `rexos-skills` crate (manifest/schema validation, loader precedence, dependency resolver)
  - Runtime skill policy + approval controls (`SessionSkillPolicy`)
  - Skill lifecycle ACP events and skill audit records (`rexos.audit.skill_runs`)
  - New CLI command group: `loopforge skills list|show|doctor|run`
  - New docs pages: skills reference and skills quickstart (EN + ZH)

### Fixed

- `loopforge onboard` no longer reports built-in starter success unless the expected artifact is actually created, and runtime JSON fallback now recognizes `function_name`-style tool calls emitted by real Ollama-compatible models.

## [1.1.0] - 2026-03-06

### Added

- `loopforge onboard --starter <hello|workspace-brief|repo-onboarding>` for guided first-day task selection.
- Workspace-local onboarding reports at `.loopforge/onboard-report.json` and `.loopforge/onboard-report.md`.
- New first-day docs pages for starter tasks and onboarding troubleshooting (EN + zh-CN).
- New blog post documenting the next LoopForge iteration informed by OpenFang and OpenClaw.

### Changed

- `loopforge doctor` now includes additive `next_actions` guidance and prints `Suggested next steps` in text output.
- Onboarding metrics reports now include recommended fixes for the most common failure categories.
- README, homepage, CLI reference, walkthrough, reliability docs, and nav now route users through `onboard` first.

## [1.0.0] - 2026-03-06

### Changed

- Hard-cut public branding and user-facing runtime surfaces from `RexOS` to `LoopForge`.
- Default config/data paths now use `~/.loopforge` and workspace artifacts use `.loopforge/`.
- Public environment variables now use the `LOOPFORGE_*` prefix; legacy `REXOS_*` guidance was removed from docs and scripts.
- Harness artifacts now use `loopforge-progress.md`, and public docs/examples now point to the `LoopForge` GitHub repo and URLs.

## [0.1.0] - Planned

### Added

- First public release baseline:
  - `rexos` CLI install/run path
  - Multi-provider model routing foundation
  - Long-running harness workflow
  - GitHub Release binary publishing pipeline
