# RexOS Runtime Decomposition Design

Date: 2026-03-07

## Goal

Improve readability of `crates/rexos-runtime/src/lib.rs` without changing external behavior.
This iteration keeps `AgentRuntime` and `OutboxDispatcher` as the primary public entry points,
but extracts internal support responsibilities into smaller modules.

## Problems

- `crates/rexos-runtime/src/lib.rs` mixes public runtime entry points, ACP persistence,
  tool-call JSON compatibility helpers, approval policy logic, and many internal record/args types.
- The current file is difficult to scan because data structures and helper functions for unrelated
  concerns are interleaved with runtime orchestration logic.
- The helper groups are good candidates for extraction because they have clear data boundaries and
  low risk of behavior change.

## Chosen Approach

Approach B: extract support modules while keeping `AgentRuntime` in `lib.rs`.

### New internal modules

- `records.rs`
  - Holds internal args/record/status types used by runtime-managed tools and persistence.
  - Re-exports `SessionSkillPolicy`, `AcpEventRecord`, and `AcpDeliveryCheckpointRecord` for the
    existing public surface.
- `tool_calls.rs`
  - Holds tool-call JSON compatibility helpers and output truncation helpers.
- `approval.rs`
  - Holds approval-mode parsing and tool/skill approval decision helpers.
- `acp.rs`
  - Holds ACP checkpoint key helpers and ACP event/checkpoint persistence helpers.

## Boundaries

`lib.rs` will keep:

- `AgentRuntime`
- `OutboxDispatcher`
- runtime orchestration methods
- public constructors and public runtime-facing methods

`lib.rs` will stop directly defining:

- bottom-of-file record/args/status structs
- JSON tool-call parsing helpers
- ACP persistence helpers
- approval decision helpers

## Compatibility

- No public CLI behavior changes.
- No change to runtime-managed tool semantics.
- `SessionSkillPolicy` remains publicly reachable as `rexos::agent::SessionSkillPolicy` through
  re-export from `lib.rs`.
- ACP storage keys and serialized payload formats remain unchanged.

## Testing Strategy

Because `rexos-runtime` currently has little or no direct unit coverage for these helpers, this
iteration adds focused unit tests inside the extracted modules before or during the move:

- `tool_calls.rs`
  - parse embedded JSON tool calls from free-form text
  - normalize wrapped tool arguments
  - truncate tool results while preserving head/tail markers
- `approval.rs`
  - detect readonly skill permission sets
  - detect approval requirement for risky tools
- `acp.rs`
  - append ACP events and enforce retention cap
  - reject empty checkpoint session ids or return defaults consistently

## Non-Goals

- No major re-architecture of `AgentRuntime` methods.
- No splitting by full business domains like agents/tasks/cron/knowledge in this iteration.
- No user-visible feature changes.

## Expected Outcome

After this iteration, `rexos-runtime/src/lib.rs` should read as a runtime orchestration file rather
than a catch-all implementation file, making the next decomposition step safer.
