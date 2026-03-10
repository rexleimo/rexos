# Bedrock Provider Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add an AWS Bedrock (Converse API) LLM provider, feature-gated in code but enabled in official release builds, with doctor + docs support.

**Architecture:** Add a new `ProviderKind::Bedrock` plus `BedrockDriver` in `rexos-llm` behind a `bedrock` feature; propagate the feature to `loopforge-cli`; update release workflows; add docs + an optional smoke test; keep public docs free of competitor references.

**Tech Stack:** Rust, tokio, AWS SDK for Rust (`aws-config`, `aws-sdk-bedrockruntime`), GitHub Actions, MkDocs.

---

### Task 1: Add Bedrock config types (kernel)

**Files:**
- Modify: `crates/rexos-kernel/src/config.rs`
- Modify: `crates/rexos-kernel/src/config/defaults.rs`
- Test: `crates/rexos-kernel/src/config/tests.rs`

**Step 1: Write failing config parse test**

Add a test that parses a minimal provider entry:

```toml
[providers.bedrock]
kind = "bedrock"
default_model = "anthropic.claude-3-5-sonnet-20241022-v2:0"

[providers.bedrock.aws_bedrock]
region = "us-east-1"
```

Expected: parse succeeds and `ProviderKind::Bedrock` is recognized.

**Step 2: Run the test to verify it fails**

Run: `cargo test -p rexos-kernel config::tests -q`
Expected: FAIL because `bedrock` is not a known `ProviderKind`.

**Step 3: Implement the config changes**

- Add `ProviderKind::Bedrock` with `#[serde(rename = "bedrock")]`.
- Add `AwsBedrockConfig` to `ProviderConfig` as `aws_bedrock: Option<AwsBedrockConfig>`.
- Define fields: `region` (default `us-east-1`), optional `cross_region`, optional `profile`.

**Step 4: Re-run tests**

Run: `cargo test -p rexos-kernel config::tests -q`
Expected: PASS.

**Step 5: Add a Bedrock provider preset**

In `default_providers()` add a `bedrock` provider entry that is present but not used by default routing:
- `kind = bedrock`
- `default_model = ""` (forces explicit model selection when users opt-in)
- `aws_bedrock.region = "us-east-1"`

**Step 6: Commit**

```bash
git add crates/rexos-kernel/src/config.rs crates/rexos-kernel/src/config/defaults.rs crates/rexos-kernel/src/config/tests.rs
git commit -m "feat(config): add bedrock provider kind and fields"
```

---

### Task 2: Add feature plumbing for Bedrock (workspace crates)

**Files:**
- Modify: `crates/rexos-llm/Cargo.toml`
- Modify: `crates/rexos/Cargo.toml`
- Modify: `crates/loopforge-cli/Cargo.toml`

**Step 1: Add optional AWS dependencies to `rexos-llm`**

- Add a `bedrock` feature
- Add optional deps: `aws-config`, `aws-sdk-bedrockruntime`, `aws-smithy-types` (and whatever minimal set is required)

**Step 2: Propagate feature to `rexos` and `loopforge-cli`**

- In `rexos`: `bedrock = ["rexos-llm/bedrock"]`
- In `loopforge-cli`: `bedrock = ["rexos/bedrock"]`

**Step 3: Verify Cargo resolves without enabling the feature**

Run: `cargo check -p loopforge-cli -q`
Expected: PASS.

**Step 4: Commit**

```bash
git add crates/rexos-llm/Cargo.toml crates/rexos/Cargo.toml crates/loopforge-cli/Cargo.toml
git commit -m "build: add bedrock feature plumbing"
```

---

### Task 3: Implement Bedrock driver (feature-gated)

**Files:**
- Create: `crates/rexos-llm/src/bedrock.rs`
- (Optional split): `crates/rexos-llm/src/bedrock/driver.rs`, `request.rs`, `response.rs`, `document.rs`
- Modify: `crates/rexos-llm/src/lib.rs`

**Step 1: Write a unit test for JSON ↔ Document conversion (feature-gated)**

Add `#[cfg(feature = "bedrock")]` tests that round-trip a `serde_json::Value`.

Run: `cargo test -p rexos-llm --features bedrock -q`
Expected: FAIL until conversion helpers exist.

**Step 2: Implement conversion helpers**

Implement:
- `json_to_document(serde_json::Value) -> aws_smithy_types::Document`
- `document_to_json(&Document) -> serde_json::Value`

**Step 3: Implement message mapping**

Convert `openai_compat::ChatMessage` list into:
- Bedrock system blocks (`SystemContentBlock`)
- Bedrock messages (`Message` with `ContentBlock`s)

Must handle:
- system extraction (joined or multiple blocks)
- tool result messages (`Role::Tool`) as `ContentBlock::ToolResult` inside a **user** message
- Bedrock alternation: merge consecutive same-role messages instead of emitting invalid sequences

**Step 4: Implement tool config mapping**

Map `openai_compat::ToolDefinition` into Bedrock `ToolConfiguration`:
- tool name + description
- JSON schema (`parameters`) converted into `Document`

**Step 5: Implement `BedrockDriver`**

Requirements:
- lazy AWS client initialization in `chat()` (async credential chain resolution)
- use configured region/profile/cross-region prefix from provider config
- call `client.converse()` with system/messages/tools
- map response:
  - `ContentBlock::Text` → assistant `content`
  - `ContentBlock::ToolUse` → `tool_calls`

**Step 6: Add module exports**

Update `crates/rexos-llm/src/lib.rs` to expose the driver when feature is enabled.

**Step 7: Compile check**

Run: `cargo check -p rexos-llm --features bedrock -q`
Expected: PASS.

**Step 8: Commit**

```bash
git add crates/rexos-llm/src crates/rexos-llm/src/lib.rs
git commit -m "feat(llm): add bedrock driver (converse api)"
```

---

### Task 4: Wire Bedrock into the LLM registry (with fallback)

**Files:**
- Modify: `crates/rexos-llm/src/registry/build.rs`
- Test: `crates/rexos-llm/src/registry/tests.rs`

**Step 1: Write failing registry tests**

- When feature **disabled**: building registry with a Bedrock provider should succeed and the driver should be `UnimplementedDriver`.
- When feature **enabled**: building registry should succeed (without contacting AWS).

**Step 2: Implement registry match arm**

In `build_driver` add:
- `ProviderKind::Bedrock`:
  - `#[cfg(feature = "bedrock")]` return `BedrockDriver`
  - `#[cfg(not(feature = "bedrock"))]` return `UnimplementedDriver("bedrock")`

**Step 3: Run tests**

Run: `cargo test -p rexos-llm registry::tests -q`
Expected: PASS.

**Step 4: Commit**

```bash
git add crates/rexos-llm/src/registry/build.rs crates/rexos-llm/src/registry/tests.rs
git commit -m "feat(llm): register bedrock provider with fallback"
```

---

### Task 5: Add doctor checks for Bedrock routing

**Files:**
- Modify: `crates/loopforge-cli/src/doctor/probes/config/runtime/router.rs`
- Modify: `crates/loopforge-cli/src/doctor/actions.rs` (if needed for next-actions)
- Test: `crates/loopforge-cli/src/doctor/tests.rs`

**Step 1: Write failing tests**

Cases:
- router points to `bedrock` and binary compiled **without** bedrock → error with rebuild hint
- router points to `bedrock` but missing region/model → warn/error with exact config keys

**Step 2: Implement probe**

Add a doctor check that:
- detects any `router.*.provider` referencing a Bedrock provider
- validates:
  - `providers.<name>.default_model` non-empty when `router.*.model = default`
  - `providers.<name>.aws_bedrock.region` non-empty
- uses `cfg!(feature = "bedrock")` to emit “compiled without support” messaging

**Step 3: Run tests**

Run: `cargo test -p loopforge-cli doctor::tests -q`
Expected: PASS.

**Step 4: Commit**

```bash
git add crates/loopforge-cli/src/doctor
git commit -m "feat(doctor): add bedrock routing checks"
```

---

### Task 6: Add Bedrock smoke test (optional, ignored)

**Files:**
- Create: `crates/rexos/tests/bedrock_smoke.rs`

**Step 1: Add an ignored test behind the feature**

- `#[cfg(feature = "bedrock")]`
- `#[tokio::test] #[ignore]`
- Reads:
  - `LOOPFORGE_BEDROCK_REGION` (default `us-east-1`)
  - `LOOPFORGE_BEDROCK_MODEL` (required)
  - AWS creds via standard SDK chain (no custom env var)
- Sends: “Reply with the single word: OK”

**Step 2: Verify it compiles**

Run: `cargo test -p rexos --features bedrock --test bedrock_smoke -- --ignored -q`
Expected: It should compile; runtime may fail unless the environment is configured.

**Step 3: Commit**

```bash
git add crates/rexos/tests/bedrock_smoke.rs
git commit -m "test: add optional bedrock smoke"
```

---

### Task 7: Add Bedrock to provider health report helper

**Files:**
- Modify: `scripts/provider_health_report.py`

**Step 1: Add a new case**

When `LOOPFORGE_BEDROCK_MODEL` is set, include:

- id: `bedrock_smoke`
- command: `LOOPFORGE_BEDROCK_REGION=... LOOPFORGE_BEDROCK_MODEL=... cargo test -p rexos --features bedrock --test bedrock_smoke -- --ignored --nocapture`

**Step 2: Dry run**

Run: `python3 scripts/provider_health_report.py --out-dir .tmp/provider-health`
Expected: includes `bedrock_smoke` as planned when env is present.

**Step 3: Commit**

```bash
git add scripts/provider_health_report.py
git commit -m "chore: add bedrock to provider health report"
```

---

### Task 8: Public docs updates (first-day packaging)

**Files:**
- Modify: `docs-site/how-to/providers.md`
- Modify: `docs-site/reference/config.md`

**Step 1: Add Bedrock provider kind + example config**

- `kind = "bedrock"`
- `aws_bedrock.region`
- `default_model` guidance
- safe verification commands (`config validate`, `doctor`, one small agent run)

**Step 2: Verify docs build**

Run: `python3 -m mkdocs build --strict`
Expected: PASS.

**Step 3: Commit**

```bash
git add docs-site/how-to/providers.md docs-site/reference/config.md
git commit -m "docs: add bedrock provider setup guide"
```

---

### Task 9: Enable Bedrock in official release builds

**Files:**
- Modify: `.github/workflows/release.yml`
- Modify: `.github/workflows/release-dry-run.yml`

**Step 1: Update build steps**

Change:
- `cargo build --release -p loopforge-cli --locked`

To:
- `cargo build --release -p loopforge-cli --locked --features bedrock`

**Step 2: Verify workflow unit tests (if present)**

If the repo has workflow tests, run them (otherwise skip).

**Step 3: Commit**

```bash
git add .github/workflows/release.yml .github/workflows/release-dry-run.yml
git commit -m "ci: build release binaries with bedrock feature"
```

---

### Task 10: Full verification (evidence before claims)

Run: `make check`
Expected: exit 0.

Optional (only when AWS is configured):

Run: `LOOPFORGE_BEDROCK_MODEL=<id> cargo test -p rexos --features bedrock --test bedrock_smoke -- --ignored --nocapture`
Expected: PASS or a clear AWS error indicating missing permissions/model access.

