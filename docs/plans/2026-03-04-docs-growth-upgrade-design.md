# Docs Growth Upgrade Design (2026-03-04)

## Context

Goal from user:
1. Compare competitor documentation and extract optimization opportunities for `meos`.
2. If needed, make docs richer for beginners with more practical code examples.
3. Optionally add a growth-oriented blog article.

Compared local snapshots:
- `openfang/` (README + docs + troubleshooting/config)
- `.tmp/openclaw/` (README + docs structure)
- `meos/docs-site/`

## Findings

### What competitors do well

1. Onboarding funnel clarity
- OpenFang and OpenClaw present strong "first run" paths and installation matrices.
- OpenClaw prominently links onboarding, wizard, FAQ, and showcase.

2. FAQ/troubleshooting depth
- OpenFang has a structured troubleshooting + FAQ page with command-level diagnostics.

3. Scenario breadth and discoverability
- OpenClaw has a very large scenario inventory and dense internal cross-linking.

### Gaps in `meos` docs (before this upgrade)

1. No dedicated beginner FAQ page.
2. Case tasks existed but lacked a compact "starter pack" of many copy/paste examples in one place.
3. No explicit growth/blog surface for positioning and traffic capture.

## Design Decision

Adopt a "three-layer docs funnel":

1. **Beginner confidence layer**  
   Add `how-to/faq.md` in English and Chinese.

2. **Execution layer**  
   Add a "10 copy/paste tasks" page in case tasks (EN + zh-CN).

3. **Growth layer**  
   Add `blog/` index + comparison article (EN + zh-CN) and wire into nav/home CTA.

## Scope

In scope:
- `mkdocs.yml` nav updates (including i18n nav labels)
- New FAQ pages
- New case-task template page
- New blog section + one comparison post
- Home page CTA updates
- Case task index updates

Out of scope:
- External SEO infra (analytics, sitemap tuning, keyword tooling)
- Product feature changes

## Verification Plan

1. Build docs site:
```bash
mkdocs build
```

2. Validate key pages resolve:
- `/how-to/faq/`
- `/examples/case-tasks/ten-copy-paste-tasks/`
- `/blog/`
- `/blog/rexos-vs-openfang-openclaw/`
- zh-CN equivalents

3. Ensure nav renders for EN + zh-CN.

## Expected Outcome

Beginners can:
- unblock themselves faster (FAQ),
- get value immediately (copy/paste tasks),
- understand product positioning quickly (blog comparison),
without reading the entire docs set first.
