# LoopForge Skills System Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 在 LoopForge 中实现可生产可扩展的 Skills 系统（发现、加载、依赖解析、权限审批、运行时注入、可观测与发布迁移），让“个人助理能力”可标准化复用与分发。

**Architecture:** 新增 `rexos-skills` 子系统作为 Skills 的核心域层，负责 manifest/schema、目录扫描、版本与依赖解析、执行策略与审计事件；`rexos-runtime` 负责执行期注入与权限闸门；`loopforge-cli` 提供 `skills` 命令面；文档与迁移策略分阶段（MVP -> Beta -> Marketplace-ready）推进，保证旧能力不回退。

**Tech Stack:** Rust workspace（serde/serde_json/toml/semver），现有 `rexos-runtime`/`rexos-kernel`/`rexos-memory`，`loopforge-cli`（clap），Rust integration tests，MkDocs 文档站。

---

### Task 1: 建立 Skill Manifest 与 Schema 校验层

**Files:**
- Create: `crates/rexos-skills/Cargo.toml`
- Create: `crates/rexos-skills/src/lib.rs`
- Create: `crates/rexos-skills/src/manifest.rs`
- Create: `crates/rexos-skills/tests/manifest_schema.rs`
- Modify: `Cargo.toml`

**Step 1: 写失败测试（manifest 解析/校验）**

```rust
#[test]
fn rejects_manifest_without_name() {
    let raw = r#"version = \"0.1.0\""#;
    let err = parse_manifest(raw).unwrap_err();
    assert!(err.to_string().contains("name"));
}
```

**Step 2: 运行测试确认红灯**

Run: `cargo test -p rexos-skills manifest_schema -- --nocapture`
Expected: FAIL（`parse_manifest` 未实现或校验不完整）。

**Step 3: 最小实现 manifest 结构与校验**

```rust
#[derive(Deserialize)]
pub struct SkillManifest {
    pub name: String,
    pub version: semver::Version,
    pub entry: String,
    pub permissions: Vec<String>,
    pub dependencies: Vec<SkillDependency>,
}
```

**Step 4: 运行测试确认绿灯**

Run: `cargo test -p rexos-skills manifest_schema -- --nocapture`
Expected: PASS。

**Step 5: Commit**

```bash
git add Cargo.toml crates/rexos-skills
git commit -m "feat(skills): add manifest schema and validation core"
```

### Task 2: 实现 Skills 目录扫描与加载器

**Files:**
- Create: `crates/rexos-skills/src/loader.rs`
- Create: `crates/rexos-skills/tests/loader_discovery.rs`
- Modify: `crates/rexos-kernel/src/paths.rs`

**Step 1: 写失败测试（多目录发现优先级）**

```rust
#[test]
fn workspace_skills_override_global_skills() {
    let resolved = discover_skills(&workspace, &global).unwrap();
    assert_eq!(resolved["write-plan"].source, SkillSource::Workspace);
}
```

**Step 2: 跑测试确认红灯**

Run: `cargo test -p rexos-skills loader_discovery -- --nocapture`
Expected: FAIL（发现顺序/覆盖规则未实现）。

**Step 3: 实现加载规则**
- 扫描目录：`<workspace>/.loopforge/skills/`、`<workspace>/.rexos/skills/`（兼容）、`$HOME/.codex/skills/`
- 规则：同名 skill 采用“workspace > home”
- 忽略无效目录并记录 warning。

**Step 4: 跑测试确认绿灯**

Run: `cargo test -p rexos-skills loader_discovery -- --nocapture`
Expected: PASS。

**Step 5: Commit**

```bash
git add crates/rexos-skills/src/loader.rs crates/rexos-skills/tests/loader_discovery.rs crates/rexos-kernel/src/paths.rs
git commit -m "feat(skills): add deterministic skill discovery and override precedence"
```

### Task 3: 加入依赖与版本解析（DAG + 约束）

**Files:**
- Create: `crates/rexos-skills/src/resolver.rs`
- Create: `crates/rexos-skills/tests/resolver_graph.rs`

**Step 1: 写失败测试（循环依赖、版本不满足）**

```rust
#[test]
fn rejects_dependency_cycle() {
    let err = resolve_order(&graph_with_cycle()).unwrap_err();
    assert!(err.to_string().contains("cycle"));
}
```

**Step 2: 跑测试确认红灯**

Run: `cargo test -p rexos-skills resolver_graph -- --nocapture`
Expected: FAIL。

**Step 3: 实现解析器**
- 基于拓扑排序输出加载顺序
- 用 `semver` 检查 `>=` / `^` 约束
- 提供明确错误信息（skill 名、约束、当前版本）。

**Step 4: 跑测试确认绿灯**

Run: `cargo test -p rexos-skills resolver_graph -- --nocapture`
Expected: PASS。

**Step 5: Commit**

```bash
git add crates/rexos-skills/src/resolver.rs crates/rexos-skills/tests/resolver_graph.rs
git commit -m "feat(skills): add dependency graph and semver resolver"
```

### Task 4: 在 Runtime 注入 Skills 并接入权限模型

**Files:**
- Modify: `crates/rexos-runtime/src/lib.rs`
- Modify: `crates/rexos-kernel/src/config.rs`
- Create: `crates/rexos/tests/runtime_skills_policy.rs`

**Step 1: 写失败测试（未授权 skill 被阻断）**

```rust
#[tokio::test]
async fn blocks_skill_when_not_approved() {
    let err = run_session_with_skill("shell-helper").await.unwrap_err();
    assert!(err.to_string().contains("not approved"));
}
```

**Step 2: 跑测试确认红灯**

Run: `cargo test -p rexos --test runtime_skills_policy -- --nocapture`
Expected: FAIL。

**Step 3: 实现权限与审批闸门**
- 新增配置：`skills.allowlist`、`skills.require_approval`、`skills.auto_approve_readonly`
- 运行时策略：未授权直接拒绝并写审计事件
- 对高风险权限（`shell`, `docker_exec`, `network`) 强制审批。

**Step 4: 跑测试确认绿灯**

Run: `cargo test -p rexos --test runtime_skills_policy -- --nocapture`
Expected: PASS。

**Step 5: Commit**

```bash
git add crates/rexos-runtime/src/lib.rs crates/rexos-kernel/src/config.rs crates/rexos/tests/runtime_skills_policy.rs
git commit -m "feat(runtime): enforce skills policy and approval gates"
```

### Task 5: 接入可观测性（events/checkpoints/audit）

**Files:**
- Modify: `crates/rexos-runtime/src/lib.rs`
- Modify: `crates/rexos-memory/src/lib.rs`
- Create: `crates/rexos/tests/skills_audit_events.rs`

**Step 1: 写失败测试（执行 skill 后必须有审计事件）**

```rust
#[tokio::test]
async fn emits_skill_events_and_audit_records() {
    let events = runtime.list_acp_events(Some(session_id), 50).unwrap();
    assert!(events.iter().any(|e| e.event_type == "skill.loaded"));
    assert!(events.iter().any(|e| e.event_type == "skill.executed"));
}
```

**Step 2: 跑测试确认红灯**

Run: `cargo test -p rexos --test skills_audit_events -- --nocapture`
Expected: FAIL。

**Step 3: 增加事件与审计落库**
- `skill.discovered`
- `skill.loaded`
- `skill.blocked`
- `skill.executed`
- `skill.failed`

**Step 4: 跑测试确认绿灯**

Run: `cargo test -p rexos --test skills_audit_events -- --nocapture`
Expected: PASS。

**Step 5: Commit**

```bash
git add crates/rexos-runtime/src/lib.rs crates/rexos-memory/src/lib.rs crates/rexos/tests/skills_audit_events.rs
git commit -m "feat(observability): add skill lifecycle events and audit trail"
```

### Task 6: CLI 能力面（list/show/doctor/run）

**Files:**
- Modify: `crates/loopforge-cli/src/main.rs`
- Create: `crates/loopforge-cli/src/skills.rs`
- Create: `crates/loopforge-cli/tests/skills_cli.rs`

**Step 1: 写失败测试（skills 子命令可见且可执行）**

```rust
#[test]
fn exposes_skills_subcommands() {
    let cmd = Cli::command();
    assert!(cmd.get_subcommands().any(|s| s.get_name() == "skills"));
}
```

**Step 2: 跑测试确认红灯**

Run: `cargo test -p loopforge-cli skills_cli -- --nocapture`
Expected: FAIL。

**Step 3: 最小实现 CLI 子命令**
- `loopforge skills list`
- `loopforge skills show <name>`
- `loopforge skills doctor`
- `loopforge skills run <name> --input ...`

**Step 4: 跑测试确认绿灯**

Run: `cargo test -p loopforge-cli skills_cli -- --nocapture`
Expected: PASS。

**Step 5: Commit**

```bash
git add crates/loopforge-cli/src/main.rs crates/loopforge-cli/src/skills.rs crates/loopforge-cli/tests/skills_cli.rs
git commit -m "feat(cli): add loopforge skills command group"
```

### Task 7: 文档与示例（小白可上手）

**Files:**
- Create: `docs-site/reference/skills.md`
- Create: `docs-site/zh-CN/reference/skills.md`
- Create: `docs-site/tutorials/skills-quickstart.md`
- Create: `docs-site/zh-CN/tutorials/skills-quickstart.md`
- Modify: `mkdocs.yml`

**Step 1: 写失败文档检查（导航与关键命令）**

Run: `python3 -m mkdocs build --strict`
Expected: FAIL（页面/导航尚未创建）。

**Step 2: 编写文档与代码样例**
- 从 0 到 1 创建 `hello-skill`
- 权限声明与审批示例
- 依赖冲突排查示例
- 失败案例与错误解释（新手向）。

**Step 3: 跑文档检查到绿灯**

Run: `python3 -m mkdocs build --strict`
Expected: PASS。

**Step 4: Commit**

```bash
git add docs-site mkdocs.yml
git commit -m "docs: add comprehensive skills reference and beginner quickstart"
```

### Task 8: 全链路验收与发布迁移

**Files:**
- Modify: `CHANGELOG.md`
- Modify: `docs/alignment.md`
- Create: `docs/plans/2026-03-05-loopforge-skills-rollout-checklist.md`

**Step 1: 运行全量验证**

Run:
- `cargo test --workspace --locked`
- `python3 -m mkdocs build --strict`

Expected: 全部 PASS。

**Step 2: 迁移策略落地**
- 标注实验特性开关：`skills.experimental=true`
- 提供升级脚本（将旧目录映射到新目录）
- 将 `docs/alignment.md` 里 “Skills system 未实现” 更新为已实现范围与缺口。

**Step 3: 发布说明**
- `CHANGELOG.md` 增加 Skills MVP/Beta 发布记录
- `docs/plans/...rollout-checklist.md` 记录回滚条件与监控项。

**Step 4: Commit**

```bash
git add CHANGELOG.md docs/alignment.md docs/plans/2026-03-05-loopforge-skills-rollout-checklist.md
git commit -m "release(skills): finalize rollout checklist and migration notes"
```

---

## Milestones

1. **MVP（本地可用）**
- 交付：Task 1-4
- 退出标准：可发现/加载 skill，能解析依赖，能在 runtime 执行并受权限控制。

2. **Beta（团队可协作）**
- 交付：Task 5-7
- 退出标准：审计事件完整、CLI 可运维、文档可让新手独立跑通。

3. **Marketplace-ready（可分发生态）**
- 交付：Task 8 + 后续签名/来源信任（可在下一期扩展）
- 退出标准：有发布迁移与回滚方案、版本兼容策略稳定、对外分发风险可控。

## Risk Checklist

- 依赖循环导致加载阻塞：必须在 resolver 阶段 fail-fast。
- 高危权限绕过：审批闸门默认 deny，审计必须可追溯。
- 文档与实现漂移：每个里程碑都绑定命令级验收。
- 兼容性回归：保留 `.rexos/skills` 读取兼容窗口，分阶段弃用。
