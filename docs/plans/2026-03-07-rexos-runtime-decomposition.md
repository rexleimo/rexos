# RexOS Runtime Decomposition Implementation Plan

Date: 2026-03-07

I'm using the writing-plans skill to create the implementation plan.

## Task 1: Add support module declarations
1. Add `mod acp;`, `mod approval;`, `mod records;`, and `mod tool_calls;` to `crates/rexos-runtime/src/lib.rs`.
2. Re-export `SessionSkillPolicy`, `AcpEventRecord`, and `AcpDeliveryCheckpointRecord` from the new module boundary.
3. Run `cargo test -p rexos-runtime --locked` and confirm the first compile failure points at missing modules.

## Task 2: Extract tool-call helpers with TDD
1. Create `crates/rexos-runtime/src/tool_calls.rs` with one failing unit test for parsing free-form embedded JSON tool calls.
2. Run `cargo test -p rexos-runtime --locked` and confirm the test fails for the expected missing helpers.
3. Move `normalize_tool_arguments`, `parse_tool_calls_from_json_content`, `into_tool_calls`, `truncate_tool_result_with_flag`, `parse_json_tool_calls_from_value`, `extract_json_tool_calls_from_text`, and `find_balanced_json_object_end` into the new module.
4. Add focused tests for argument normalization and truncation behavior.
5. Run `cargo test -p rexos-runtime --locked` and confirm the module tests pass.

## Task 3: Extract approval helpers with TDD
1. Create `crates/rexos-runtime/src/approval.rs` with one failing unit test for readonly permission detection.
2. Run `cargo test -p rexos-runtime --locked` and confirm the failure is expected.
3. Move `ApprovalMode`, `tool_requires_approval`, `json_bool_field`, `tool_approval_is_granted`, `skill_approval_is_granted`, and `skill_permissions_are_readonly` into the new module.
4. Add focused tests for approval-gated and readonly tool scenarios.
5. Run `cargo test -p rexos-runtime --locked` and confirm the approval tests pass.

## Task 4: Extract ACP helpers with TDD
1. Create `crates/rexos-runtime/src/acp.rs` with one failing unit test for ACP event retention.
2. Run `cargo test -p rexos-runtime --locked` and confirm the failure is expected.
3. Move ACP key and persistence helpers into the new module.
4. Add tests for checkpoint round-tripping and event retention ordering.
5. Run `cargo test -p rexos-runtime --locked` and confirm the ACP tests pass.

## Task 5: Extract record and args definitions
1. Create `crates/rexos-runtime/src/records.rs` and move internal args/record/status definitions out of `lib.rs`.
2. Keep visibility minimal using `pub(crate)` except for public policy or ACP types that must stay exported.
3. Update `lib.rs` imports to use the new record module.
4. Run `cargo test -p rexos-runtime --locked` and fix any visibility or serde attribute regressions.

## Task 6: Run targeted regression verification
1. Run `cargo test -p rexos-runtime --locked`.
2. Run `cargo test -p rexos --locked runtime_skills_policy reserved_channel_tools reserved_task_tools reserved_cron_tools` if supported, otherwise run `cargo test -p rexos --locked` for the closest relevant coverage.
3. Summarize file size and readability improvements before handing off.
