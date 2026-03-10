# AWS Bedrock Provider Design (Converse API)

**Status:** approved  
**Date:** 2026-03-10

## Goal

Add an **AWS Bedrock** LLM provider to LoopForge using the native **Converse API**, while keeping the repository lightweight and the first-day experience clear.

This iteration follows the agreed approach:

- implement Bedrock behind a Cargo feature gate (`bedrock`) to keep AWS SDK deps optional
- **official release binaries** are built **with** `--features bedrock` so Bedrock works out of the box

## Scope (P0)

### In scope

- New provider kind: `kind = "bedrock"` (native AWS SDK)
- Feature gating across crates:
  - `loopforge-cli --features bedrock` enables Bedrock end-to-end
- `loopforge doctor` improvements for Bedrock:
  - detect when routing targets Bedrock but binary was compiled without Bedrock support
  - validate Bedrock-specific config fields (region/model)
- Docs updates (public, user-facing):
  - add Bedrock to provider list + configuration examples
  - include a “shortest verification path” for Bedrock setup
- Optional real-provider smoke test (`#[ignore]`, feature-gated)
- Provider health script includes Bedrock when env is present

### Non-goals (this iteration)

- Streaming responses
- Fine-grained token accounting (Bedrock billing is external)
- Full “MCP transport” work (P1)
- End-to-end multimodal UX (P2)

## User experience (first-day)

### Default behavior

- `loopforge init` writes a default config that includes a Bedrock provider preset, but routing continues to default to `ollama`.
- Users opt-in by switching one route (usually `router.coding`) to `"bedrock"`.

### Minimal Bedrock setup path

1. Ensure AWS auth works via standard AWS mechanisms (env vars, profile, SSO, instance role).
2. Configure:
   - Bedrock region
   - Bedrock model ID (as `default_model`, or set `router.*.model` explicitly)
3. Verify:
   - `loopforge config validate`
   - `loopforge doctor`
   - run one small agent task

## Configuration design

### Provider kind

Add `ProviderKind::Bedrock` with TOML value `bedrock`.

### Provider fields

Bedrock uses the standard `providers.*` block plus an additional optional nested config for Bedrock specifics:

```toml
[providers.bedrock]
kind = "bedrock"
base_url = ""          # unused
api_key_env = ""       # unused (AWS SDK credential chain)
default_model = ""     # required when router.*.model = "default"

[providers.bedrock.aws_bedrock]
region = "us-east-1"
cross_region = ""      # optional: "us" | "eu" | "apac" | "global"
profile = ""           # optional: AWS shared config profile name
```

Notes:
- Credentials are **not** stored in config. We rely on the AWS SDK default credential chain.
- `cross_region` (when set) is applied as a prefix (`us.` / `eu.` / …) to the model ID used in requests.

## Runtime architecture

### Driver shape

Add `BedrockDriver` in `rexos-llm` implementing the existing `LlmDriver` trait:

- Input: `openai_compat::ChatCompletionRequest`
- Output: `openai_compat::ChatMessage` (with optional `tool_calls`)

The driver:
- lazily creates an AWS Bedrock Runtime client on first request (async credential resolution)
- calls `client.converse()` with:
  - extracted system blocks
  - converted messages (including tool results)
  - tool configuration derived from the `tools` list
- maps Converse output blocks into:
  - assistant text
  - tool calls (`tool_use`) → `tool_calls`

### Behavior when feature is disabled

If the binary is built without `--features bedrock`:
- the registry still loads config successfully
- the Bedrock provider resolves to an `UnimplementedDriver("bedrock")`
- `doctor` surfaces an actionable error if routing targets Bedrock

## Docs & public boundary

- Bedrock documentation belongs in `docs-site/` and must **not** mention competitor projects.
- Competitor references remain internal-only under `docs/internal/` and `docs/plans/`.

## Testing strategy

- Unit tests:
  - config parsing/serialization for `kind = "bedrock"` and nested `aws_bedrock`
  - registry behavior when Bedrock feature is disabled (should not hard-fail startup)
- Optional integration:
  - `bedrock_smoke` test (ignored by default) behind the `bedrock` feature
  - `scripts/provider_health_report.py` exposes a Bedrock case when `LOOPFORGE_BEDROCK_MODEL` is set

## Release / CI

- Release workflows build `loopforge-cli` with `--features bedrock`.
- Release packaging and smoke steps remain the same (we still run `loopforge doctor --json` in the packaged binary).

## Risks

- AWS SDK dependency weight and compile time (mitigated by feature gating).
- Bedrock tool-use semantics differ from OpenAI/Anthropic; message conversion needs careful handling.

## Follow-ups (not P0)

- P1: MCP transport abstraction
- P2: End-to-end multimodal (image) support

