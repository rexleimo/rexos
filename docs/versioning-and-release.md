# Versioning and Release Policy

## Goals

- Define the first release (`v1.0.0`) with a clear scope.
- Keep future version bumps planned and predictable.
- Enforce one rule: if this iteration must bump version, the same commit/PR must include both version number changes and changelog updates.

## Versioning Model

This repo uses SemVer with a `v` tag prefix (`vMAJOR.MINOR.PATCH`):

- `MAJOR`: incompatible public behavior changes.
- `MINOR`: planned feature iteration (preferred during `0.x` stage).
- `PATCH`: bugfixes, documentation, or small safe improvements.

Current workspace version is `1.1.0` (from root `Cargo.toml` `[workspace.package].version`).

## First Release Plan (`v1.0.0`)

Release target:
- Core CLI path is runnable (`loopforge init`, `loopforge agent run`).
- Existing multi-provider routing and harness flow are stable enough for first external users.
- GitHub Release binary workflow is available (tag-triggered).

Release checklist:
1. Run full test suite: `cargo test`.
2. Confirm release packaging script works locally:
   `python3 scripts/package_release.py --version v1.0.0 --target local --bin target/release/loopforge --out-dir dist`
3. Ensure `CHANGELOG.md` contains a `1.0.0` section.
4. Create and push tag:
   `git tag v1.0.0 && git push origin v1.0.0`

## Mandatory Rule for Version-Bump Iterations

When the maintainer explicitly says this iteration needs a version bump, the delivery must include all of the following in the same change set:

1. Version number update:
   - Root `Cargo.toml` `[workspace.package].version`
   - Any docs/examples that include hardcoded version text
2. Changelog update:
   - Add or update the target version section in `CHANGELOG.md`
   - Include concise notes for user-visible changes

If either item is missing, iteration is not considered releasable.

## Planned Iteration Workflow

1. Confirm iteration scope and choose target version (`MINOR` or `PATCH`).
2. Implement changes normally.
3. If iteration is marked "needs version bump", update version + changelog together.
4. Run verification (`cargo test`, plus release packaging smoke check when release-bound).
5. Merge, then cut tag (`vX.Y.Z`).

## Changelog Format

`CHANGELOG.md` follows this structure:

- `## [Unreleased]`
- `## [X.Y.Z] - YYYY-MM-DD`
  - `### Added`
  - `### Changed`
  - `### Fixed`

Only user-visible changes should be recorded.
