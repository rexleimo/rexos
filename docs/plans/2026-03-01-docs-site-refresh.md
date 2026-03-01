# Docs Site Refresh Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make `os.rexai.top` a polished, practical RexOS documentation site (better UI + deeper docs + real use cases), with English/中文 parity.

**Architecture:** Keep MkDocs Material. Improve `mkdocs.yml` theme/features, add small CSS/JS overrides, rewrite the landing pages, and expand “use cases / providers / troubleshooting” into concrete, copy-pastable workflows.

**Tech Stack:** MkDocs (`mkdocs.yml`), MkDocs Material, Markdown + Material components, Mermaid (via CDN).

---

### Task 1: Improve MkDocs theme + capabilities

**Files:**
- Modify: `mkdocs.yml`
- Create: `docs-site/assets/stylesheets/extra.css`
- Create: `docs-site/assets/javascripts/mermaid-init.js`

**Step 1: Enable richer markdown + Material features**

- Add `attr_list`, `md_in_html`, `pymdownx.tabbed`, `pymdownx.emoji` (Material icons), and optional Mermaid custom fence.
- Add `edit_uri` and `extra` (social links).
- Add `extra_css` and `extra_javascript` for site polish.

**Step 2: Verify docs build**

Run: `python3 -m mkdocs build --strict`  
Expected: `INFO - Documentation built ...` with exit code `0`.

---

### Task 2: Rewrite the landing pages (EN/ZH)

**Files:**
- Modify: `docs-site/index.md`
- Modify: `docs-site/zh/index.md`

**Step 1: Add “hero” + cards + CTAs**

- Use Material “cards grid” + buttons.
- Add a simple Mermaid architecture diagram: harness loop + memory + tools + routing.
- Link prominently to Quickstart / Harness / Providers / Use cases / Security.

**Step 2: Verify links**

Run: `python3 -m mkdocs build --strict`  
Expected: no broken links.

---

### Task 3: Expand “Use Cases” into practical recipes (EN/ZH)

**Files:**
- Modify: `docs-site/how-to/use-cases.md`
- Modify: `docs-site/zh/how-to/use-cases.md`

**Step 1: Replace list-only content with real recipes**

Include at least:
- “Fix a failing test suite with harness checkpoints”
- “Mechanical edit across a repo safely”
- “Provider routing: local planning on Ollama + cloud coding”
- “Daemon integration / healthz / automation”

Each recipe should contain:
- goal + prerequisites
- exact commands (`rexos init`, `rexos agent run`, `rexos harness init/run`)
- expected artifacts and how to review/rollback

**Step 2: Verify docs build**

Run: `python3 -m mkdocs build --strict`

---

### Task 4: Add troubleshooting + provider-native deep dives (EN/ZH)

**Files:**
- Create: `docs-site/how-to/troubleshooting.md`
- Create: `docs-site/zh/how-to/troubleshooting.md`
- Modify: `docs-site/how-to/providers.md`
- Modify: `docs-site/zh/how-to/providers.md`
- Modify: `mkdocs.yml` (nav)

**Step 1: Troubleshooting content**

Cover:
- “Seeing `GitHub Pages is designed to host…` / 404” → enable Pages (Source=GitHub Actions) then rerun workflow
- Ollama connection issues (`127.0.0.1:11434`)
- Windows-specific notes (PowerShell script, path quirks)

**Step 2: Provider-native details**

- Add GLM (Zhipu native) + MiniMax native examples in both languages.
- Clarify API key env var expectations and “model string is provider-defined”.

**Step 3: Verify docs build**

Run: `python3 -m mkdocs build --strict`

---

### Task 5: Commit + push

**Step 1: Commit**

Run:
```bash
git add mkdocs.yml docs-site docs/plans/2026-03-01-docs-site-refresh.md
git commit -m "docs: improve docs site UI and add use cases"
```

**Step 2: Push**

Run: `git push origin main`

