# Internal Playbook: OpenClaw Trend Response (Do Not Publish)

Date: 2026-03-05
Audience: internal team only

## Goal

Use the OpenClaw trend window to promote LoopForge without drifting from our core positioning:
engineering delivery reliability.

## External vs Internal Rule

- Can publish externally:
  - case tasks
  - X copy materials in `docs/marketing/`
- Internal only (do not publish to docs-site/blog):
  - competitor analysis reports
  - growth playbooks
  - tactical execution checklists

## Market Signals (from OpenClaw public assets)

1. Strong default onboarding entry (`openclaw onboard`) lowers first-run friction.
2. Broad scenario docs reduce evaluation time.
3. Frequent docs/release updates build trust loops.

## LoopForge Action Priorities

### P0 (1-2 weeks)

1. Keep publishing onboarding reliability evidence (weekly snapshot).
2. Expand scenario packs for engineering tasks (migration, test repair, release readiness).
3. Pair each release note with 2 short X posts and 1 thread opener.

### P1 (2-4 weeks)

1. Add "failure -> fix -> rerun" tutorial flow.
2. Build proof gallery with command + artifact evidence.
3. Track weekly baseline metrics (TTFS, first-task success, failure category top N).

## X Materials

- English: `docs/marketing/openclaw-trend-x-posts.en.txt`
- Chinese: `docs/marketing/openclaw-trend-x-posts.zh-CN.txt`

These files are external-facing materials and can be posted directly.

## Internal Publishing Checklist

1. Run X length lint before posting:
   `python3 scripts/x_post_lint.py --file <material-file> --limit 280 --warn-at 260`
2. Remove internal process lines from public-facing copy.
3. Ensure no internal-only reports are linked from docs-site.
