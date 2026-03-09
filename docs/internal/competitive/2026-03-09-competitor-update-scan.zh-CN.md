> Internal-only note: retained for technical reference; do not publish from `docs-site/` or link from `mkdocs.yml`.
>
> 更新范围：本机镜像 `openfang/`、`ironclaw/`、`.tmp/openclaw/` 在 2026-03-09 拉到最新后的“可借鉴点”快速扫描。

# 2026-03-09 竞品更新扫描（OpenClaw / OpenFang / IronClaw）

## 本次扫描目的

在不改变 LoopForge（`meos/`）产品定位与安全边界的前提下，收集最近上游项目的新改动中**值得借鉴的工程实践/安全护栏/可靠性语义**，并给出简短的 Adopt/Extend/Build 建议。

> 注：这里只记录“我们能学到什么”，不把竞品内容带入公开 docs。

## 已同步到的版本（本机）

- OpenClaw：`f2f561fab`（tag 见 `v2026.3.8`, `v2026.3.8-beta.1` 等）
- IronClaw：`d73e35c`（`feat: add AWS Bedrock LLM provider via native Converse API`）
- OpenFang：`385aee8`（`fix streaming`；本地改动已先 stash）

## OpenClaw：近期值得借鉴的点

### 1) Skills 安装下载的“根目录固定 + 归一化 + 反穿越”策略

证据：
- commit：`9abf014f3`（`fix(skills): pin validated download roots`）
- 文件：`.tmp/openclaw/src/agents/skills-install-download.ts`
- 关键点：下载 `targetDir` 强制落在 per-skill tools root 下；`realpath` 后再拼相对路径；落盘写入通过 “rootDir + relativePath” 安全写 API；配套 zip-slip / tar-slip 测试覆盖。

对 LoopForge 的映射：
- 如果/当 LoopForge 支持**远程 skills 获取**（或 skills 依赖下载），建议采用同样的“**canonical root + relative path**”落盘策略，并把归档解压做成强约束（防 `../`、软链接逃逸）。

建议：**Extend（未来）**

### 2) Secrets/供应链：repo 级 secrets 扫描基线 + CI/预提交护栏

证据：
- `.tmp/openclaw/.detect-secrets.cfg`、`.tmp/openclaw/.secrets.baseline`
- `.tmp/openclaw/.pre-commit-config.yaml`
- `.tmp/openclaw/.github/workflows/codeql.yml`

对 LoopForge 的映射：
- LoopForge 已有运行时泄漏防护（leak-guard），但 repo 层面仍可能误提交密钥。
- 可考虑增加轻量“**提交前/CI secrets 扫描**”护栏（不替代 leak-guard，而是防止密钥进入 git 历史）。

建议：**Adopt（可做成 DX/安全护栏类迭代）**

### 3) Scheduling：重启 catch-up / missed cron 的“削峰”语义

证据：
- commit：`96d17f3cb` / `79853aca9`（`stagger missed cron jobs on restart`）
- CHANGELOG：`Cron/restart catch-up semantics` 等多条修复

对 LoopForge 的映射：
- LoopForge 已有 `scheduling/cron` 与持久化记录；如果未来引入“重启后补跑 missed 任务”，建议默认加削峰（例如 jitter、限速、批次执行），避免启动瞬间触发大量 backlog 把 runtime/模型/外部服务压垮。

建议：**Extend（中期）**

### 4) ACP provenance（来源元数据）与可见回执

证据：
- CHANGELOG：`ACP/Provenance`（`openclaw acp --provenance ...`）

对 LoopForge 的映射：
- LoopForge 已有 ACP events / audit records；可借鉴“来源元数据 + 可见 receipt”作为审计与可追踪性增强（尤其在跨系统输入时）。

建议：**Compose（与现有 records/outbox/audit 结合）**

## IronClaw：近期值得借鉴的点

### 1) 新增 AWS Bedrock 原生 provider（Converse API）

证据：
- commit：`d73e35c`（`feat: add AWS Bedrock LLM provider via native Converse API`）

对 LoopForge 的映射：
- LoopForge 当前 provider 覆盖 OpenAI-compat + Anthropic/Gemini + 多家国产；
- 若用户有 Bedrock 需求，可按类似方式增加 `bedrock` driver，并在 `doctor`/`config validate` 增强环境变量/凭证检查。

建议：**Build（需求驱动）**

### 2) MCP：transport 抽象 + stdio/UDS + OAuth 修复

证据：
- commit：`02f85a8`（`feat(mcp): transport abstraction, stdio/UDS transports, and OAuth fixes`）

对 LoopForge 的映射：
- LoopForge 文档里有 MCP 文章，但当前代码侧未见实现；
- 若要做“真正的 MCP 客户端/服务端对接”，可借鉴其 transport 分层，避免一开始就把进程/stdio/socket/OAuth 全糊在一处。

建议：**Build（立项时优先参考）**

### 3) 多模态：全渠道 image support

证据：
- commit：`553c306`（`feat: full image support across all channels`）

对 LoopForge 的映射：
- LoopForge 已有 media 工具链（inspect/generate/transcribe）；但“端到端多模态”（输入、存储、工具返回、审计、展示）仍可对照其覆盖策略补齐缺口。

建议：**Extend（以当前 media ops 为底座）**

## OpenFang：近期值得借鉴的点（偏修复/加固）

证据（log）：
- `385aee8` fix streaming
- `a00327a` fix auth
- `4667f49` fix csp
- `f241394` shell hardening
- `9e230f4` security hardening

对 LoopForge 的映射：
- 如果 LoopForge daemon / web surfaces 后续增加 UI 或对外接口，可以参考它们在 auth/CSP/shell hardening 上的变更方向；
- 但当前 LoopForge 侧更紧急的是“安全边界 + DX 护栏 + 可靠性语义”体系化。

建议：**Adopt（按需挑选具体项）**

## 结论：建议近期优先借鉴的 3 项

1) **Repo secrets 扫描护栏**（detect-secrets/gitleaks + baseline）：避免密钥进入 git 历史（DX/安全类，低风险）
2) **Skills 下载/解压安全**（若/当引入远程 skills）：canonical root + archive traversal 测试
3) **Cron catch-up 削峰语义**：为未来 scheduling 扩展预留“安全默认值”

## Adopt / Extend / Build 备忘

- Adopt：repo secrets 扫描护栏、（将来）某些 CI 安全检查
- Extend：cron 削峰、端到端多模态覆盖、ACP provenance 记录
- Build：Bedrock provider、MCP transport（若立项）
