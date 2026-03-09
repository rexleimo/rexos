# Secrets Scanning Guardrails Design (CI)

**Status:** implemented
**Date:** 2026-03-09

## Goal

Reduce the risk of committing API keys / tokens by adding an automated secret-scanning guardrail in CI.

## Non-goals

- Not a replacement for runtime leak-guard / secret redaction.
- Not a full history rewrite solution; if a secret is found, the correct response is still to **rotate** it.
- No new “always-on” local pre-commit requirement (CI is the baseline).

## Context / reference

Competitor scan (internal): `docs/internal/competitive/2026-03-09-competitor-update-scan.zh-CN.md`.

OpenClaw uses detect-secrets baselines and pre-commit hooks. For LoopForge we start with a CI-first approach that is simple to adopt and easy to evolve.

## Options considered

### Option A: CI `gitleaks` (recommended)

Pros:
- Fast to adopt, minimal repository changes
- Runs on PR/push in CI (no local setup required)
- Widely used secret scanner with configurable rules

Cons:
- Can produce false positives; may need allowlist config as the repo grows

### Option B: `detect-secrets` baseline + pre-commit

Pros:
- Strong workflow for managing false positives with a baseline
- Hooks can prevent leaks before they reach CI

Cons:
- Higher setup cost (baseline generation, pre-commit install, developer education)

## Decision

Start with **Option A**:

- Add a `gitleaks` job to `.github/workflows/ci.yml`
- Add an optional local target `make secrets-check` that runs `gitleaks` when installed

If we later hit noise, add a repo-local `gitleaks.toml` allowlist and/or introduce `detect-secrets` baseline as an incremental upgrade.

## Verification

- Local: `make check` (fmt + tests + docs)
- CI: `gitleaks` job runs on PR/push
