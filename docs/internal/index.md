# Internal Docs Index

This folder contains **repository-internal** documentation.

- Do **not** wire anything under `docs/internal/` into `docs-site/` or `mkdocs.yml`.
- Public docs are built only from `docs-site/`.

## Maintainership quick links

### Boundaries

- [Public docs boundary (what can be published)](public-docs-boundary.md)

### Runtime maps

- [Runtime module map (post-readability refactor)](runtime-module-map.md)

### OKRs

- [2026-03 Readability OKR (zh-CN)](2026-03-readability-okr.zh-CN.md)

### Security (internal)

- [Network + egress security notes](loopforge-network-security.md)

### Plans and references

- [Implementation plans](../plans/)
- [Competitive archive](competitive/)

## Authoring rules (internal)

- Prefer **dated** filenames for plans/notes: `YYYY-MM-DD-<topic>.md`.
- Keep “user-path” docs in `docs-site/`; keep maintainer maps, strategy, and competitor notes here.
- When a doc is bilingual, put the language in the filename (example: `*.zh-CN.md`).
