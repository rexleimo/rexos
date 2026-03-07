# RexOS Tools Definition Decomposition Design

Date: 2026-03-07

## Goal

Improve readability of `crates/rexos-tools/src/lib.rs` by extracting the tool registration layer
(parameter structs plus tool schema definitions) into domain-focused modules, without changing tool
behavior.

## Problems

- `crates/rexos-tools/src/lib.rs` still holds dozens of `*Args` structs and a long run of `*_def()`
  functions after the main execution logic.
- The registration layer is conceptually separate from tool execution, but both currently live in the
  same file.
- Readers have to scroll through a giant schema table before getting back to actual runtime logic.

## Chosen Approach

Approach B: split parameter definitions and tool schema registration by tool domain while keeping
execution logic inside `lib.rs` for this iteration.

### New internal modules

- `defs/mod.rs`
  - top-level aggregation and re-exports for arg structs used by `Toolset::call`
- `defs/fs.rs`
  - file read/write/list args and tool defs
- `defs/process.rs`
  - shell/process/docker args and defs
- `defs/web.rs`
  - web fetch, PDF, search, A2A args and defs
- `defs/media.rs`
  - media/image/canvas args and defs
- `defs/browser.rs`
  - browser action args and defs
- `defs/compat.rs`
  - compatibility alias defs

## Boundaries

`lib.rs` keeps:

- `Toolset`
- process/browser session execution logic
- path validation and runtime helpers
- network safety helpers
- top-level `call()` dispatch

`lib.rs` stops directly defining:

- `*Args` request structs
- `*_def()` tool schema builders
- compat alias schema registration

## Compatibility

- No tool names change.
- No schema shape changes.
- No execution behavior changes.
- Existing `Toolset::definitions()` output remains stable.

## Testing Strategy

Add focused module tests for the new defs layer:

- aggregated core defs include representative tools from multiple domains
- compat defs include alias tools such as `file_read` and `apply_patch`
- browser defs still advertise browser primitives

Then run the existing `rexos-tools` test suite to verify runtime behavior did not regress.

## Non-Goals

- Do not move browser/process/media execution logic in this iteration.
- Do not redesign tool schemas.
- Do not change private/public network policy behavior.

## Expected Outcome

After this iteration, `rexos-tools/src/lib.rs` reads more like the execution engine, while the
registration layer is isolated in a dedicated `defs/` subtree that can be evolved independently.
