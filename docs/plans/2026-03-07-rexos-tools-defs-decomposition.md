# RexOS Tools Definition Decomposition Implementation Plan

Date: 2026-03-07

I'm using the writing-plans skill to create the implementation plan.

## Task 1: Add defs module boundary
1. Add `mod defs;` to `crates/rexos-tools/src/lib.rs`.
2. Create `crates/rexos-tools/src/defs/mod.rs` with one failing unit test that expects aggregated tool defs to include representative names.
3. Run `cargo test -p rexos-tools --locked` and confirm the failure is caused by missing defs helpers.

## Task 2: Extract filesystem and process defs
1. Create `crates/rexos-tools/src/defs/fs.rs` and move file-related args plus `fs_read_def`/`fs_write_def`.
2. Create `crates/rexos-tools/src/defs/process.rs` and move shell/process/docker args plus matching tool defs.
3. Re-export the moved arg structs from `defs/mod.rs`.
4. Update `Toolset::call()` and `Toolset::definitions()` to compile against the new module boundary.
5. Run `cargo test -p rexos-tools --locked`.

## Task 3: Extract web and media defs
1. Create `crates/rexos-tools/src/defs/web.rs` and move web/PDF/search/A2A args plus tool defs.
2. Create `crates/rexos-tools/src/defs/media.rs` and move image/media/canvas args plus tool defs.
3. Re-export the moved arg structs from `defs/mod.rs`.
4. Run `cargo test -p rexos-tools --locked`.

## Task 4: Extract browser defs and compat defs
1. Create `crates/rexos-tools/src/defs/browser.rs` and move browser args plus browser tool defs.
2. Create `crates/rexos-tools/src/defs/compat.rs` and move compatibility alias defs.
3. Update the defs aggregator in `defs/mod.rs` so `Toolset::definitions()` no longer directly references individual `*_def()` helpers from `lib.rs`.
4. Run `cargo test -p rexos-tools --locked`.

## Task 5: Clean lib.rs and verify readability improvement
1. Remove the old arg/def blocks from `crates/rexos-tools/src/lib.rs`.
2. Keep only imports/re-exports needed for execution logic.
3. Run `rustfmt --edition 2021` on the touched files.
4. Run `cargo test -p rexos-tools --locked` again.

## Task 6: Run adjacent regression coverage
1. Run `cargo test -p rexos --locked --test reserved_channel_tools` if any tool registration use is indirectly exercised there.
2. If a more direct adjacent suite exists, prefer that specific test target; otherwise stop at `rexos-tools` coverage.
3. Summarize which domains were extracted and how much `lib.rs` shrank.
