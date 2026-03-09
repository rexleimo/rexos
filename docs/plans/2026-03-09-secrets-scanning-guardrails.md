# Secrets Scanning Guardrails Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add CI secret scanning (gitleaks) and a small local convenience target.

**Architecture:** Keep it additive and isolated: a new CI job in `ci.yml`, plus a `Makefile` target that fails with a clear message when gitleaks isn’t installed.

**Tech Stack:** GitHub Actions, Make, gitleaks.

---

### Task 1: Add CI `gitleaks` job

**Files:**
- Modify: `.github/workflows/ci.yml`

**Step 1: Update workflow**

Add a new job that:
- checks out with `fetch-depth: 0`
- runs `gitleaks/gitleaks-action@v2`
- passes `GITHUB_TOKEN`
- disables PR comments by default

**Step 2: Verify workflow unit tests**

Run: `python3 -m unittest scripts.tests.test_ci_workflows`
Expected: PASS.

**Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add gitleaks secret scan"
```

---

### Task 2: Add local `make secrets-check`

**Files:**
- Modify: `Makefile`
- Modify: `AGENTS.md` (optional, document new target)

**Step 1: Add target**

Add `secrets-check`:
- require `gitleaks` in PATH
- run `gitleaks detect --source . --no-git`

**Step 2: Verification**

Run: `make help`
Expected: includes `secrets-check`.

Run: `make secrets-check`
Expected: either PASS (if gitleaks installed) or FAIL with install hint (if not installed).

**Step 3: Commit**

```bash
git add Makefile AGENTS.md
git commit -m "chore: add secrets-check target"
```

---

### Task 3: Full verification

Run: `make check`
Expected: exit 0.
