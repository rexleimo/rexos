use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context};

use rexos_kernel::router::{ModelRouter, TaskKind};
use rexos_kernel::security::SecurityConfig;
use rexos_llm::driver::LlmDriver;
use rexos_llm::openai_compat::{ChatCompletionRequest, ChatMessage, Role};
use rexos_llm::registry::LlmRegistry;
use rexos_memory::MemoryStore;
use rexos_tools::Toolset;

tokio::task_local! {
    static AGENT_CALL_DEPTH: std::cell::Cell<usize>;
}

const MAX_AGENT_CALL_DEPTH: usize = 4;
const MAX_TOOL_RESULT_CHARS: usize = 15_000;
const TOOL_AUDIT_KEY: &str = "rexos.audit.tool_calls";
const TOOL_AUDIT_MAX_RECORDS: usize = 2_000;
const SKILL_AUDIT_KEY: &str = "rexos.audit.skill_runs";
const SKILL_AUDIT_MAX_RECORDS: usize = 2_000;
const SESSION_ALLOWED_TOOLS_KEY_PREFIX: &str = "rexos.sessions.allowed_tools.";
const SESSION_ALLOWED_SKILLS_KEY_PREFIX: &str = "rexos.sessions.allowed_skills.";
const SESSION_SKILL_POLICY_KEY_PREFIX: &str = "rexos.sessions.skill_policy.";
const ACP_EVENTS_KEY: &str = "rexos.acp.events";
const ACP_EVENTS_MAX_RECORDS: usize = 5_000;
const ACP_CHECKPOINTS_KEY_PREFIX: &str = "rexos.acp.checkpoints.";

mod acp;
mod approval;
mod leak_guard;
mod records;
mod tool_calls;

use acp::{
    acp_delivery_checkpoints_get, acp_delivery_checkpoints_set, acp_events_get, append_acp_event,
};
use approval::{
    skill_approval_is_granted, skill_permissions_are_readonly, tool_approval_is_granted,
    tool_requires_approval, ApprovalMode,
};
use leak_guard::{LeakGuard, LeakGuardAudit, LeakGuardVerdict};
pub use records::{AcpDeliveryCheckpointRecord, AcpEventRecord, SessionSkillPolicy};
use records::{
    AgentFindToolArgs, AgentKillToolArgs, AgentRecord, AgentSendToolArgs, AgentSpawnToolArgs,
    AgentStatus, ChannelSendToolArgs, CronCancelToolArgs, CronCreateToolArgs, CronJobRecord,
    EventPublishToolArgs, EventRecord, HandActivateToolArgs, HandDeactivateToolArgs, HandDef,
    HandInstanceRecord, HandInstanceStatus, HandStatusToolArgs, KnowledgeAddEntityToolArgs,
    KnowledgeAddRelationToolArgs, KnowledgeEntityRecord, KnowledgeQueryToolArgs,
    KnowledgeRelationRecord, MemoryRecallToolArgs, MemoryStoreToolArgs, OutboxMessageRecord,
    OutboxStatus, ScheduleCreateToolArgs, ScheduleDeleteToolArgs, ScheduleRecord, SkillAuditRecord,
    TaskClaimToolArgs, TaskCompleteToolArgs, TaskListToolArgs, TaskPostToolArgs, TaskRecord,
    TaskStatus, ToolAuditRecord, WorkflowRunStateRecord, WorkflowRunToolArgs,
    WorkflowStepStateRecord,
};
use tool_calls::{
    normalize_tool_arguments, parse_tool_calls_from_json_content, truncate_tool_result_with_flag,
};

fn tool_event_payload(
    tool_name: &str,
    truncated: Option<bool>,
    error: Option<&str>,
    reason: Option<&str>,
    leak_guard: Option<&LeakGuardAudit>,
) -> serde_json::Value {
    let mut payload = serde_json::Map::new();
    payload.insert(
        "tool".to_string(),
        serde_json::Value::String(tool_name.to_string()),
    );
    if let Some(truncated) = truncated {
        payload.insert("truncated".to_string(), serde_json::Value::Bool(truncated));
    }
    if let Some(error) = error {
        payload.insert(
            "error".to_string(),
            serde_json::Value::String(error.to_string()),
        );
    }
    if let Some(reason) = reason {
        payload.insert(
            "reason".to_string(),
            serde_json::Value::String(reason.to_string()),
        );
    }
    if let Some(leak_guard) = leak_guard {
        if let Ok(value) = serde_json::to_value(leak_guard) {
            payload.insert("leak_guard".to_string(), value);
        }
    }
    serde_json::Value::Object(payload)
}

#[derive(Debug)]
pub struct AgentRuntime {
    memory: MemoryStore,
    llms: LlmRegistry,
    router: ModelRouter,
    security: SecurityConfig,
    leak_guard: LeakGuard,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OutboxDrainSummary {
    pub sent: u32,
    pub failed: u32,
}

#[derive(Debug)]
pub struct OutboxDispatcher {
    memory: MemoryStore,
    http: reqwest::Client,
}

impl OutboxDispatcher {
    pub fn new(memory: MemoryStore) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .context("build http client")?;
        Ok(Self { memory, http })
    }

    pub async fn drain_once(&self, limit: usize) -> anyhow::Result<OutboxDrainSummary> {
        let mut msgs = self.outbox_messages_get()?;
        let mut summary = OutboxDrainSummary::default();

        let mut processed = 0usize;
        for msg in msgs.iter_mut() {
            if processed >= limit.max(1) {
                break;
            }
            if msg.status != OutboxStatus::Queued {
                continue;
            }
            processed += 1;

            let now = AgentRuntime::now_epoch_seconds();
            msg.attempts = msg.attempts.saturating_add(1);
            msg.updated_at = now;
            msg.last_error = None;

            let result = match msg.channel.as_str() {
                "console" => {
                    self.deliver_console(msg);
                    Ok(())
                }
                "webhook" => self.deliver_webhook(msg).await,
                other => Err(anyhow::anyhow!("unknown channel: {other}")),
            };

            match result {
                Ok(()) => {
                    msg.status = OutboxStatus::Sent;
                    msg.sent_at = Some(now);
                    summary.sent = summary.sent.saturating_add(1);
                    if let Some(session_id) = msg.session_id.as_deref() {
                        let _ = self.upsert_acp_delivery_checkpoint(
                            session_id,
                            &msg.channel,
                            &msg.message_id,
                        );
                        let _ = append_acp_event(
                            &self.memory,
                            AcpEventRecord {
                                id: uuid::Uuid::new_v4().to_string(),
                                session_id: Some(session_id.to_string()),
                                event_type: "delivery.sent".to_string(),
                                payload: serde_json::json!({
                                    "channel": msg.channel.clone(),
                                    "message_id": msg.message_id.clone(),
                                    "recipient": msg.recipient.clone(),
                                }),
                                created_at: now,
                            },
                        );
                    }
                }
                Err(e) => {
                    msg.status = OutboxStatus::Failed;
                    msg.last_error = Some(e.to_string());
                    summary.failed = summary.failed.saturating_add(1);
                    if let Some(session_id) = msg.session_id.as_deref() {
                        let _ = append_acp_event(
                            &self.memory,
                            AcpEventRecord {
                                id: uuid::Uuid::new_v4().to_string(),
                                session_id: Some(session_id.to_string()),
                                event_type: "delivery.failed".to_string(),
                                payload: serde_json::json!({
                                    "channel": msg.channel.clone(),
                                    "message_id": msg.message_id.clone(),
                                    "recipient": msg.recipient.clone(),
                                    "error": msg.last_error.clone(),
                                }),
                                created_at: now,
                            },
                        );
                    }
                }
            }
        }

        if processed > 0 {
            self.outbox_messages_set(&msgs)?;
        }

        Ok(summary)
    }

    fn outbox_messages_get(&self) -> anyhow::Result<Vec<OutboxMessageRecord>> {
        let raw = self
            .memory
            .kv_get("rexos.outbox.messages")
            .context("kv_get rexos.outbox.messages")?
            .unwrap_or_else(|| "[]".to_string());
        Ok(serde_json::from_str(&raw).unwrap_or_default())
    }

    fn outbox_messages_set(&self, msgs: &[OutboxMessageRecord]) -> anyhow::Result<()> {
        let raw = serde_json::to_string(msgs).context("serialize rexos.outbox.messages")?;
        self.memory
            .kv_set("rexos.outbox.messages", &raw)
            .context("kv_set rexos.outbox.messages")?;
        Ok(())
    }

    fn upsert_acp_delivery_checkpoint(
        &self,
        session_id: &str,
        channel: &str,
        cursor: &str,
    ) -> anyhow::Result<()> {
        let mut checkpoints = acp_delivery_checkpoints_get(&self.memory, session_id)?;
        let now = AgentRuntime::now_epoch_seconds();
        if let Some(existing) = checkpoints.iter_mut().find(|c| c.channel == channel) {
            existing.cursor = cursor.to_string();
            existing.updated_at = now;
        } else {
            checkpoints.push(AcpDeliveryCheckpointRecord {
                channel: channel.to_string(),
                cursor: cursor.to_string(),
                updated_at: now,
            });
        }
        acp_delivery_checkpoints_set(&self.memory, session_id, &checkpoints)
    }

    fn deliver_console(&self, msg: &OutboxMessageRecord) {
        let subject = msg.subject.as_deref().unwrap_or("");
        println!(
            "[rexos][channel_send][console] to={} subject={} message={}",
            msg.recipient, subject, msg.message
        );
    }

    async fn deliver_webhook(&self, msg: &OutboxMessageRecord) -> anyhow::Result<()> {
        let url = std::env::var("LOOPFORGE_WEBHOOK_URL")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("LOOPFORGE_WEBHOOK_URL is not set"))?;

        let payload = serde_json::json!({
            "message_id": msg.message_id,
            "recipient": msg.recipient,
            "subject": msg.subject,
            "message": msg.message,
            "created_at": msg.created_at,
        });

        let resp = self
            .http
            .post(url)
            .json(&payload)
            .send()
            .await
            .context("send webhook request")?;

        if !resp.status().is_success() {
            bail!("webhook returned http {}", resp.status());
        }
        Ok(())
    }
}

impl AgentRuntime {
    pub fn new(memory: MemoryStore, llms: LlmRegistry, router: ModelRouter) -> Self {
        Self::new_with_security_config(memory, llms, router, SecurityConfig::default())
    }

    pub fn new_with_security_config(
        memory: MemoryStore,
        llms: LlmRegistry,
        router: ModelRouter,
        security: SecurityConfig,
    ) -> Self {
        let leak_guard = LeakGuard::from_security(&security);
        Self {
            memory,
            llms,
            router,
            security,
            leak_guard,
        }
    }

    pub async fn run_session(
        &self,
        workspace_root: PathBuf,
        session_id: &str,
        system_prompt: Option<&str>,
        user_prompt: &str,
        kind: TaskKind,
    ) -> anyhow::Result<String> {
        let allowed_tools = self.load_session_allowed_tools(session_id)?;
        let allowed_lookup: Option<HashSet<String>> = allowed_tools
            .as_ref()
            .map(|tools| tools.iter().cloned().collect());
        let tools = Toolset::new_with_allowed_tools_and_security(
            workspace_root.clone(),
            allowed_tools,
            self.security.clone(),
        )?;
        let provider = self.router.provider_for(kind);
        let model = self.resolve_model(provider, kind)?;

        let driver = self
            .llms
            .driver(provider)
            .ok_or_else(|| anyhow::anyhow!("unknown provider: {provider}"))?;

        let mut messages = self
            .memory
            .list_chat_messages(session_id)
            .context("load session history")?;

        if let Some(system_prompt) = system_prompt {
            let has_system = messages.iter().any(|m| m.role == Role::System);
            if !has_system {
                let system_msg = ChatMessage {
                    role: Role::System,
                    content: Some(system_prompt.to_string()),
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                };
                self.memory.append_chat_message(session_id, &system_msg)?;
                messages.push(system_msg);
            }
        }

        let user_msg = ChatMessage {
            role: Role::User,
            content: Some(user_prompt.to_string()),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        };
        self.memory.append_chat_message(session_id, &user_msg)?;
        messages.push(user_msg);
        let _ = self.append_acp_event(AcpEventRecord {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: Some(session_id.to_string()),
            event_type: "session.started".to_string(),
            payload: serde_json::json!({
                "kind": format!("{kind:?}").to_lowercase(),
                "user_prompt_chars": user_prompt.chars().count(),
            }),
            created_at: Self::now_epoch_seconds(),
        });

        let tool_defs = tools.definitions();
        let mut tool_call_counts: HashMap<String, u32> = HashMap::new();
        for _ in 0..8usize {
            let req = ChatCompletionRequest {
                model: model.clone(),
                messages: messages.clone(),
                tools: tool_defs.clone(),
                temperature: Some(0.0),
            };

            let assistant = self
                .driver_chat(&*driver, req)
                .await
                .context("llm chat completion")?;

            self.memory.append_chat_message(session_id, &assistant)?;
            messages.push(assistant.clone());

            let tool_calls = match assistant.tool_calls.clone() {
                Some(calls) if !calls.is_empty() => calls,
                _ => match assistant
                    .content
                    .as_deref()
                    .and_then(parse_tool_calls_from_json_content)
                {
                    Some(calls) => calls,
                    None => {
                        let out = assistant.content.unwrap_or_default();
                        let _ = self.append_acp_event(AcpEventRecord {
                            id: uuid::Uuid::new_v4().to_string(),
                            session_id: Some(session_id.to_string()),
                            event_type: "session.completed".to_string(),
                            payload: serde_json::json!({
                                "output_chars": out.chars().count(),
                                "reason": "assistant_stop",
                            }),
                            created_at: Self::now_epoch_seconds(),
                        });
                        return Ok(out);
                    }
                },
            };

            for call in tool_calls {
                let sig = format!("{}|{}", call.function.name, call.function.arguments);
                let count = tool_call_counts.entry(sig.clone()).or_insert(0);
                *count += 1;
                if *count >= 3 {
                    bail!("tool loop detected: {sig}");
                }

                let started_at = std::time::Instant::now();
                if let Some(allowed) = allowed_lookup.as_ref() {
                    if !allowed.contains(call.function.name.as_str()) {
                        let err =
                            format!("tool not allowed for this session: {}", call.function.name);
                        let _ = self.append_acp_event(AcpEventRecord {
                            id: uuid::Uuid::new_v4().to_string(),
                            session_id: Some(session_id.to_string()),
                            event_type: "tool.blocked".to_string(),
                            payload: serde_json::json!({
                                "tool": call.function.name.clone(),
                                "reason": "session_whitelist",
                            }),
                            created_at: Self::now_epoch_seconds(),
                        });
                        let _ = self.append_tool_audit(ToolAuditRecord {
                            session_id: session_id.to_string(),
                            tool_name: call.function.name.clone(),
                            success: false,
                            duration_ms: started_at.elapsed().as_millis() as u64,
                            truncated: false,
                            error: Some(err.clone()),
                            leak_guard: None,
                            created_at: Self::now_epoch_seconds(),
                        });
                        bail!(err);
                    }
                }

                let args_json =
                    normalize_tool_arguments(&call.function.name, &call.function.arguments);
                if let Some(warning) =
                    self.evaluate_tool_approval(session_id, &call.function.name, &args_json, false)?
                {
                    let _ = self.append_acp_event(AcpEventRecord {
                        id: uuid::Uuid::new_v4().to_string(),
                        session_id: Some(session_id.to_string()),
                        event_type: "approval.warn".to_string(),
                        payload: serde_json::json!({
                            "tool": call.function.name.clone(),
                            "message": warning,
                        }),
                        created_at: Self::now_epoch_seconds(),
                    });
                }
                let _ = self.append_acp_event(AcpEventRecord {
                    id: uuid::Uuid::new_v4().to_string(),
                    session_id: Some(session_id.to_string()),
                    event_type: "tool.started".to_string(),
                    payload: serde_json::json!({
                        "tool": call.function.name.clone(),
                    }),
                    created_at: Self::now_epoch_seconds(),
                });
                let output_result: anyhow::Result<String> = async {
                    let output = match call.function.name.as_str() {
                        "memory_store" => {
                            let args: MemoryStoreToolArgs = serde_json::from_str(&args_json)
                                .context("parse memory_store args")?;
                            self.memory
                                .kv_set(&args.key, &args.value)
                                .context("memory_store kv_set")?;
                            "ok".to_string()
                        }
                        "memory_recall" => {
                            let args: MemoryRecallToolArgs = serde_json::from_str(&args_json)
                                .context("parse memory_recall args")?;
                            self.memory
                                .kv_get(&args.key)
                                .context("memory_recall kv_get")?
                                .unwrap_or_default()
                        }
                        "agent_spawn" => {
                            let args: AgentSpawnToolArgs = serde_json::from_str(&args_json)
                                .context("parse agent_spawn args")?;
                            self.agent_spawn(args).context("agent_spawn")?
                        }
                        "agent_list" => self.agent_list().context("agent_list")?,
                        "agent_find" => {
                            let args: AgentFindToolArgs = serde_json::from_str(&args_json)
                                .context("parse agent_find args")?;
                            self.agent_find(&args.query).context("agent_find")?
                        }
                        "agent_kill" => {
                            let args: AgentKillToolArgs = serde_json::from_str(&args_json)
                                .context("parse agent_kill args")?;
                            self.agent_kill(&args.agent_id).context("agent_kill")?
                        }
                        "agent_send" => {
                            let args: AgentSendToolArgs = serde_json::from_str(&args_json)
                                .context("parse agent_send args")?;
                            self.agent_send(workspace_root.clone(), kind, args)
                                .await
                                .context("agent_send")?
                        }
                        "hand_list" => self.hand_list().context("hand_list")?,
                        "hand_activate" => {
                            let args: HandActivateToolArgs = serde_json::from_str(&args_json)
                                .context("parse hand_activate args")?;
                            self.hand_activate(args).context("hand_activate")?
                        }
                        "hand_status" => {
                            let args: HandStatusToolArgs = serde_json::from_str(&args_json)
                                .context("parse hand_status args")?;
                            self.hand_status(&args.hand_id).context("hand_status")?
                        }
                        "hand_deactivate" => {
                            let args: HandDeactivateToolArgs = serde_json::from_str(&args_json)
                                .context("parse hand_deactivate args")?;
                            self.hand_deactivate(&args.instance_id)
                                .context("hand_deactivate")?
                        }
                        "task_post" => {
                            let args: TaskPostToolArgs =
                                serde_json::from_str(&args_json).context("parse task_post args")?;
                            self.task_post(args).context("task_post")?
                        }
                        "task_list" => {
                            let args: TaskListToolArgs =
                                serde_json::from_str(&args_json).context("parse task_list args")?;
                            self.task_list(args.status.as_deref())
                                .context("task_list")?
                        }
                        "task_claim" => {
                            let args: TaskClaimToolArgs = serde_json::from_str(&args_json)
                                .context("parse task_claim args")?;
                            self.task_claim(args.agent_id.as_deref())
                                .context("task_claim")?
                        }
                        "task_complete" => {
                            let args: TaskCompleteToolArgs = serde_json::from_str(&args_json)
                                .context("parse task_complete args")?;
                            self.task_complete(&args.task_id, &args.result)
                                .context("task_complete")?
                        }
                        "event_publish" => {
                            let args: EventPublishToolArgs = serde_json::from_str(&args_json)
                                .context("parse event_publish args")?;
                            self.event_publish(args).context("event_publish")?
                        }
                        "schedule_create" => {
                            let args: ScheduleCreateToolArgs = serde_json::from_str(&args_json)
                                .context("parse schedule_create args")?;
                            self.schedule_create(args).context("schedule_create")?
                        }
                        "schedule_list" => self.schedule_list().context("schedule_list")?,
                        "schedule_delete" => {
                            let args: ScheduleDeleteToolArgs = serde_json::from_str(&args_json)
                                .context("parse schedule_delete args")?;
                            self.schedule_delete(&args.id).context("schedule_delete")?
                        }
                        "cron_create" => {
                            let args: CronCreateToolArgs = serde_json::from_str(&args_json)
                                .context("parse cron_create args")?;
                            self.cron_create(args).context("cron_create")?
                        }
                        "cron_list" => self.cron_list().context("cron_list")?,
                        "cron_cancel" => {
                            let args: CronCancelToolArgs = serde_json::from_str(&args_json)
                                .context("parse cron_cancel args")?;
                            self.cron_cancel(&args.job_id).context("cron_cancel")?
                        }
                        "channel_send" => {
                            let args: ChannelSendToolArgs = serde_json::from_str(&args_json)
                                .context("parse channel_send args")?;
                            self.channel_send(Some(session_id), args)
                                .context("channel_send")?
                        }
                        "workflow_run" => {
                            let args: WorkflowRunToolArgs = serde_json::from_str(&args_json)
                                .context("parse workflow_run args")?;
                            self.workflow_run(&workspace_root, session_id, kind, args)
                                .await
                                .context("workflow_run")?
                        }
                        "knowledge_add_entity" => {
                            let args: KnowledgeAddEntityToolArgs = serde_json::from_str(&args_json)
                                .context("parse knowledge_add_entity args")?;
                            self.knowledge_add_entity(args)
                                .context("knowledge_add_entity")?
                        }
                        "knowledge_add_relation" => {
                            let args: KnowledgeAddRelationToolArgs =
                                serde_json::from_str(&args_json)
                                    .context("parse knowledge_add_relation args")?;
                            self.knowledge_add_relation(args)
                                .context("knowledge_add_relation")?
                        }
                        "knowledge_query" => {
                            let args: KnowledgeQueryToolArgs = serde_json::from_str(&args_json)
                                .context("parse knowledge_query args")?;
                            self.knowledge_query(&args.query)
                                .context("knowledge_query")?
                        }
                        _ => tools
                            .call(&call.function.name, &args_json)
                            .await
                            .with_context(|| format!("tool {}", call.function.name))?,
                    };
                    Ok(output)
                }
                .await;

                let duration_ms = started_at.elapsed().as_millis() as u64;
                let (output, leak_guard) = match output_result {
                    Ok(output) => match self.leak_guard.inspect_tool_output(output) {
                        LeakGuardVerdict::Allowed { content, audit } => (content, audit),
                        LeakGuardVerdict::Blocked { error, audit } => {
                            let _ = self.append_acp_event(AcpEventRecord {
                                id: uuid::Uuid::new_v4().to_string(),
                                session_id: Some(session_id.to_string()),
                                event_type: "tool.blocked".to_string(),
                                payload: tool_event_payload(
                                    &call.function.name,
                                    None,
                                    Some(&error),
                                    Some("leak_guard"),
                                    Some(&audit),
                                ),
                                created_at: Self::now_epoch_seconds(),
                            });
                            let _ = self.append_tool_audit(ToolAuditRecord {
                                session_id: session_id.to_string(),
                                tool_name: call.function.name.clone(),
                                success: false,
                                duration_ms,
                                truncated: false,
                                error: Some(error.clone()),
                                leak_guard: Some(audit),
                                created_at: Self::now_epoch_seconds(),
                            });
                            bail!(error);
                        }
                    },
                    Err(e) => {
                        let err_text = e.to_string();
                        let (safe_error, leak_guard) =
                            match self.leak_guard.inspect_tool_output(err_text) {
                                LeakGuardVerdict::Allowed { content, audit } => (content, audit),
                                LeakGuardVerdict::Blocked { error, audit } => (error, Some(audit)),
                            };
                        let _ = self.append_acp_event(AcpEventRecord {
                            id: uuid::Uuid::new_v4().to_string(),
                            session_id: Some(session_id.to_string()),
                            event_type: "tool.failed".to_string(),
                            payload: tool_event_payload(
                                &call.function.name,
                                None,
                                Some(&safe_error),
                                None,
                                leak_guard.as_ref(),
                            ),
                            created_at: Self::now_epoch_seconds(),
                        });
                        let _ = self.append_tool_audit(ToolAuditRecord {
                            session_id: session_id.to_string(),
                            tool_name: call.function.name.clone(),
                            success: false,
                            duration_ms,
                            truncated: false,
                            error: Some(safe_error.clone()),
                            leak_guard,
                            created_at: Self::now_epoch_seconds(),
                        });
                        return Err(anyhow::anyhow!(safe_error));
                    }
                };

                let (output, truncated) =
                    truncate_tool_result_with_flag(output, MAX_TOOL_RESULT_CHARS);
                let _ = self.append_tool_audit(ToolAuditRecord {
                    session_id: session_id.to_string(),
                    tool_name: call.function.name.clone(),
                    success: true,
                    duration_ms,
                    truncated,
                    error: None,
                    leak_guard: leak_guard.clone(),
                    created_at: Self::now_epoch_seconds(),
                });
                let _ = self.append_acp_event(AcpEventRecord {
                    id: uuid::Uuid::new_v4().to_string(),
                    session_id: Some(session_id.to_string()),
                    event_type: "tool.succeeded".to_string(),
                    payload: tool_event_payload(
                        &call.function.name,
                        Some(truncated),
                        None,
                        None,
                        leak_guard.as_ref(),
                    ),
                    created_at: Self::now_epoch_seconds(),
                });

                let tool_msg = ChatMessage {
                    role: Role::Tool,
                    content: Some(output),
                    name: Some(call.function.name),
                    tool_call_id: Some(call.id),
                    tool_calls: None,
                };
                self.memory.append_chat_message(session_id, &tool_msg)?;
                messages.push(tool_msg);
            }
        }

        let _ = self.append_acp_event(AcpEventRecord {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: Some(session_id.to_string()),
            event_type: "session.failed".to_string(),
            payload: serde_json::json!({
                "reason": "max_iterations_exceeded",
            }),
            created_at: Self::now_epoch_seconds(),
        });
        bail!("max iterations exceeded")
    }

    fn resolve_model(&self, provider: &str, kind: TaskKind) -> anyhow::Result<String> {
        let configured = self.router.model_for(kind).trim();
        if configured.is_empty() || configured.eq_ignore_ascii_case("default") {
            let model = self
                .llms
                .default_model(provider)
                .ok_or_else(|| anyhow::anyhow!("provider missing default_model: {provider}"))?;
            Ok(model.to_string())
        } else {
            Ok(configured.to_string())
        }
    }

    async fn driver_chat(
        &self,
        driver: &dyn LlmDriver,
        req: ChatCompletionRequest,
    ) -> anyhow::Result<ChatMessage> {
        driver.chat(req).await
    }

    fn now_epoch_seconds() -> i64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    fn session_allowed_tools_key(session_id: &str) -> String {
        format!("{SESSION_ALLOWED_TOOLS_KEY_PREFIX}{session_id}")
    }

    fn session_allowed_skills_key(session_id: &str) -> String {
        format!("{SESSION_ALLOWED_SKILLS_KEY_PREFIX}{session_id}")
    }

    fn session_skill_policy_key(session_id: &str) -> String {
        format!("{SESSION_SKILL_POLICY_KEY_PREFIX}{session_id}")
    }

    pub fn set_session_allowed_tools(
        &self,
        session_id: &str,
        tools: Vec<String>,
    ) -> anyhow::Result<()> {
        let mut deduped = Vec::new();
        let mut seen = HashSet::new();
        for tool in tools {
            let tool = tool.trim().to_string();
            if tool.is_empty() {
                continue;
            }
            if seen.insert(tool.clone()) {
                deduped.push(tool);
            }
        }
        let raw = serde_json::to_string(&deduped).context("serialize session allowed tools")?;
        self.memory
            .kv_set(&Self::session_allowed_tools_key(session_id), &raw)
            .context("kv_set session allowed tools")?;
        Ok(())
    }

    fn load_session_allowed_tools(&self, session_id: &str) -> anyhow::Result<Option<Vec<String>>> {
        let raw = self
            .memory
            .kv_get(&Self::session_allowed_tools_key(session_id))
            .context("kv_get session allowed tools")?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        let parsed: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();
        let mut cleaned = Vec::new();
        let mut seen = HashSet::new();
        for tool in parsed {
            let tool = tool.trim().to_string();
            if tool.is_empty() {
                continue;
            }
            if seen.insert(tool.clone()) {
                cleaned.push(tool);
            }
        }
        if cleaned.is_empty() {
            return Ok(None);
        }
        Ok(Some(cleaned))
    }

    pub fn set_session_allowed_skills(
        &self,
        session_id: &str,
        skills: Vec<String>,
    ) -> anyhow::Result<()> {
        let mut deduped = Vec::new();
        let mut seen = HashSet::new();
        for skill in skills {
            let skill = skill.trim().to_string();
            if skill.is_empty() {
                continue;
            }
            if seen.insert(skill.clone()) {
                deduped.push(skill);
            }
        }
        let raw = serde_json::to_string(&deduped).context("serialize session allowed skills")?;
        self.memory
            .kv_set(&Self::session_allowed_skills_key(session_id), &raw)
            .context("kv_set session allowed skills")?;
        Ok(())
    }

    fn load_session_allowed_skills(&self, session_id: &str) -> anyhow::Result<Option<Vec<String>>> {
        let raw = self
            .memory
            .kv_get(&Self::session_allowed_skills_key(session_id))
            .context("kv_get session allowed skills")?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        let parsed: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();
        let mut cleaned = Vec::new();
        let mut seen = HashSet::new();
        for skill in parsed {
            let skill = skill.trim().to_string();
            if skill.is_empty() {
                continue;
            }
            if seen.insert(skill.clone()) {
                cleaned.push(skill);
            }
        }
        if cleaned.is_empty() {
            return Ok(None);
        }
        Ok(Some(cleaned))
    }

    pub fn set_session_skill_policy(
        &self,
        session_id: &str,
        policy: SessionSkillPolicy,
    ) -> anyhow::Result<()> {
        let raw = serde_json::to_string(&policy).context("serialize session skill policy")?;
        self.memory
            .kv_set(&Self::session_skill_policy_key(session_id), &raw)
            .context("kv_set session skill policy")?;
        Ok(())
    }

    fn load_session_skill_policy(&self, session_id: &str) -> anyhow::Result<SessionSkillPolicy> {
        let raw = self
            .memory
            .kv_get(&Self::session_skill_policy_key(session_id))
            .context("kv_get session skill policy")?;
        let Some(raw) = raw else {
            return Ok(SessionSkillPolicy::default());
        };
        let policy: SessionSkillPolicy = serde_json::from_str(&raw).unwrap_or_default();
        Ok(policy)
    }

    pub fn record_skill_discovered(
        &self,
        session_id: &str,
        skill_name: &str,
        source: &str,
        version: &str,
    ) -> anyhow::Result<()> {
        self.append_acp_event(AcpEventRecord {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: Some(session_id.to_string()),
            event_type: "skill.discovered".to_string(),
            payload: serde_json::json!({
                "skill": skill_name,
                "source": source,
                "version": version,
            }),
            created_at: Self::now_epoch_seconds(),
        })
    }

    pub fn authorize_skill(
        &self,
        session_id: &str,
        skill_name: &str,
        requested_permissions: &[String],
    ) -> anyhow::Result<()> {
        if let Some(allowed_skills) = self.load_session_allowed_skills(session_id)? {
            if !allowed_skills
                .iter()
                .any(|s| s.eq_ignore_ascii_case(skill_name.trim()))
            {
                let msg = format!("skill not allowed for this session: {skill_name}");
                let _ = self.append_acp_event(AcpEventRecord {
                    id: uuid::Uuid::new_v4().to_string(),
                    session_id: Some(session_id.to_string()),
                    event_type: "skill.blocked".to_string(),
                    payload: serde_json::json!({
                        "skill": skill_name,
                        "reason": "session_whitelist",
                        "message": msg,
                    }),
                    created_at: Self::now_epoch_seconds(),
                });
                bail!("{msg}");
            }
        }

        let policy = self.load_session_skill_policy(session_id)?;
        if !policy.allowlist.is_empty()
            && !policy
                .allowlist
                .iter()
                .any(|s| s.eq_ignore_ascii_case(skill_name.trim()))
        {
            let msg = format!("skill blocked by policy allowlist: {skill_name}");
            let _ = self.append_acp_event(AcpEventRecord {
                id: uuid::Uuid::new_v4().to_string(),
                session_id: Some(session_id.to_string()),
                event_type: "skill.blocked".to_string(),
                payload: serde_json::json!({
                    "skill": skill_name,
                    "reason": "policy_allowlist",
                    "message": msg,
                }),
                created_at: Self::now_epoch_seconds(),
            });
            bail!("{msg}");
        }

        if policy.require_approval
            && !(policy.auto_approve_readonly
                && skill_permissions_are_readonly(requested_permissions))
            && !skill_approval_is_granted(skill_name)
        {
            let msg = format!(
                "approval required for skill `{skill_name}` (set LOOPFORGE_SKILL_APPROVAL_ALLOW={skill_name} or all)"
            );
            let _ = self.append_acp_event(AcpEventRecord {
                id: uuid::Uuid::new_v4().to_string(),
                session_id: Some(session_id.to_string()),
                event_type: "skill.blocked".to_string(),
                payload: serde_json::json!({
                    "skill": skill_name,
                    "reason": "approval_required",
                    "message": msg,
                }),
                created_at: Self::now_epoch_seconds(),
            });
            bail!("{msg}");
        }

        self.append_acp_event(AcpEventRecord {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: Some(session_id.to_string()),
            event_type: "skill.loaded".to_string(),
            payload: serde_json::json!({
                "skill": skill_name,
                "permissions": requested_permissions,
            }),
            created_at: Self::now_epoch_seconds(),
        })?;
        Ok(())
    }

    pub fn record_skill_execution(
        &self,
        session_id: &str,
        skill_name: &str,
        requested_permissions: &[String],
        success: bool,
        error: Option<&str>,
    ) -> anyhow::Result<()> {
        let event_type = if success {
            "skill.executed"
        } else {
            "skill.failed"
        };
        self.append_acp_event(AcpEventRecord {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: Some(session_id.to_string()),
            event_type: event_type.to_string(),
            payload: serde_json::json!({
                "skill": skill_name,
                "permissions": requested_permissions,
                "error": error,
            }),
            created_at: Self::now_epoch_seconds(),
        })?;

        self.append_skill_audit(SkillAuditRecord {
            session_id: session_id.to_string(),
            skill_name: skill_name.to_string(),
            success,
            permissions: requested_permissions.to_vec(),
            error: error.map(|e| e.to_string()),
            created_at: Self::now_epoch_seconds(),
        })
    }

    fn append_tool_audit(&self, record: ToolAuditRecord) -> anyhow::Result<()> {
        let raw = self
            .memory
            .kv_get(TOOL_AUDIT_KEY)
            .context("kv_get tool audit")?
            .unwrap_or_else(|| "[]".to_string());
        let mut records: Vec<ToolAuditRecord> = serde_json::from_str(&raw).unwrap_or_default();
        records.push(record);
        if records.len() > TOOL_AUDIT_MAX_RECORDS {
            records.drain(0..(records.len() - TOOL_AUDIT_MAX_RECORDS));
        }
        let serialized = serde_json::to_string(&records).context("serialize tool audit")?;
        self.memory
            .kv_set(TOOL_AUDIT_KEY, &serialized)
            .context("kv_set tool audit")?;
        Ok(())
    }

    fn append_skill_audit(&self, record: SkillAuditRecord) -> anyhow::Result<()> {
        let raw = self
            .memory
            .kv_get(SKILL_AUDIT_KEY)
            .context("kv_get skill audit")?
            .unwrap_or_else(|| "[]".to_string());
        let mut records: Vec<SkillAuditRecord> = serde_json::from_str(&raw).unwrap_or_default();
        records.push(record);
        if records.len() > SKILL_AUDIT_MAX_RECORDS {
            records.drain(0..(records.len() - SKILL_AUDIT_MAX_RECORDS));
        }
        let serialized = serde_json::to_string(&records).context("serialize skill audit")?;
        self.memory
            .kv_set(SKILL_AUDIT_KEY, &serialized)
            .context("kv_set skill audit")?;
        Ok(())
    }

    fn append_acp_event(&self, record: AcpEventRecord) -> anyhow::Result<()> {
        append_acp_event(&self.memory, record)
    }

    pub fn list_acp_events(
        &self,
        session_id: Option<&str>,
        limit: usize,
    ) -> anyhow::Result<Vec<AcpEventRecord>> {
        let mut events = acp_events_get(&self.memory)?;
        if let Some(session_id) = session_id {
            let session_id = session_id.trim();
            if !session_id.is_empty() {
                events.retain(|e| e.session_id.as_deref() == Some(session_id));
            }
        }
        let wanted = limit.max(1);
        if events.len() > wanted {
            events = events.split_off(events.len() - wanted);
        }
        Ok(events)
    }

    pub fn list_acp_delivery_checkpoints(
        &self,
        session_id: &str,
    ) -> anyhow::Result<Vec<AcpDeliveryCheckpointRecord>> {
        acp_delivery_checkpoints_get(&self.memory, session_id)
    }

    fn evaluate_tool_approval(
        &self,
        session_id: &str,
        tool_name: &str,
        arguments_json: &str,
        explicit_gate: bool,
    ) -> anyhow::Result<Option<String>> {
        let mode = ApprovalMode::from_env();
        if mode == ApprovalMode::Off {
            return Ok(None);
        }
        if !tool_requires_approval(tool_name, arguments_json, explicit_gate) {
            return Ok(None);
        }
        if tool_approval_is_granted(tool_name) {
            return Ok(None);
        }

        let msg = format!(
            "approval required for dangerous tool `{tool_name}` (set LOOPFORGE_APPROVAL_ALLOW={tool_name} or all)"
        );
        match mode {
            ApprovalMode::Warn => Ok(Some(msg)),
            ApprovalMode::Enforce => {
                let _ = self.append_acp_event(AcpEventRecord {
                    id: uuid::Uuid::new_v4().to_string(),
                    session_id: Some(session_id.to_string()),
                    event_type: "approval.blocked".to_string(),
                    payload: serde_json::json!({
                        "tool": tool_name,
                        "message": msg,
                    }),
                    created_at: Self::now_epoch_seconds(),
                });
                bail!("{msg}")
            }
            ApprovalMode::Off => Ok(None),
        }
    }

    async fn workflow_run(
        &self,
        workspace_root: &PathBuf,
        session_id: &str,
        _kind: TaskKind,
        args: WorkflowRunToolArgs,
    ) -> anyhow::Result<String> {
        if args.steps.is_empty() {
            bail!("workflow_run requires at least one step");
        }

        let workflow_id = args
            .workflow_id
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let now = Self::now_epoch_seconds();
        let mut state = WorkflowRunStateRecord {
            workflow_id: workflow_id.clone(),
            name: args.name.clone(),
            session_id: session_id.to_string(),
            status: "running".to_string(),
            created_at: now,
            updated_at: now,
            completed_at: None,
            steps: args
                .steps
                .iter()
                .enumerate()
                .map(|(idx, step)| WorkflowStepStateRecord {
                    index: idx,
                    name: step.name.clone(),
                    tool: step.tool.clone(),
                    arguments: step.arguments.clone(),
                    status: "pending".to_string(),
                    output: None,
                    error: None,
                    started_at: None,
                    completed_at: None,
                })
                .collect(),
        };
        let state_path = workflow_state_path(workspace_root, &workflow_id);
        self.write_workflow_state(&state_path, &state)?;

        let allowed_tools = self.load_session_allowed_tools(session_id)?;
        let tools = Toolset::new_with_allowed_tools_and_security(
            workspace_root.clone(),
            allowed_tools,
            self.security.clone(),
        )?;
        let continue_on_error = args.continue_on_error.unwrap_or(false);
        let mut failed_steps = 0usize;

        let _ = self.append_acp_event(AcpEventRecord {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: Some(session_id.to_string()),
            event_type: "workflow.started".to_string(),
            payload: serde_json::json!({
                "workflow_id": workflow_id,
                "steps": state.steps.len(),
            }),
            created_at: Self::now_epoch_seconds(),
        });

        for (idx, step) in args.steps.iter().enumerate() {
            let started_at = Self::now_epoch_seconds();
            {
                let st = &mut state.steps[idx];
                st.status = "running".to_string();
                st.started_at = Some(started_at);
                st.completed_at = None;
                st.error = None;
            }
            state.updated_at = started_at;
            self.write_workflow_state(&state_path, &state)?;

            let args_json = if step.arguments.is_null() {
                "{}".to_string()
            } else {
                serde_json::to_string(&step.arguments)
                    .context("serialize workflow step arguments")?
            };

            let step_res: anyhow::Result<String> = async {
                if is_runtime_managed_tool(&step.tool) {
                    bail!(
                        "workflow step tool `{}` is runtime-managed and not supported in workflow_run yet",
                        step.tool
                    );
                }

                if let Some(warning) = self.evaluate_tool_approval(
                    session_id,
                    &step.tool,
                    &args_json,
                    step.approval_required.unwrap_or(false),
                )? {
                    let _ = self.append_acp_event(AcpEventRecord {
                        id: uuid::Uuid::new_v4().to_string(),
                        session_id: Some(session_id.to_string()),
                        event_type: "approval.warn".to_string(),
                        payload: serde_json::json!({
                            "tool": step.tool,
                            "message": warning,
                            "workflow_id": workflow_id,
                            "step_index": idx,
                        }),
                        created_at: Self::now_epoch_seconds(),
                    });
                }

                tools
                    .call(&step.tool, &args_json)
                    .await
                    .with_context(|| format!("workflow step {} ({})", idx, step.tool))
            }
            .await;

            let completed_at = Self::now_epoch_seconds();
            let st = &mut state.steps[idx];
            st.completed_at = Some(completed_at);

            match step_res {
                Ok(output) => {
                    let (output, _) = truncate_tool_result_with_flag(output, 4_000);
                    st.status = "succeeded".to_string();
                    st.output = Some(output);
                    st.error = None;
                    let _ = self.append_acp_event(AcpEventRecord {
                        id: uuid::Uuid::new_v4().to_string(),
                        session_id: Some(session_id.to_string()),
                        event_type: "workflow.step_succeeded".to_string(),
                        payload: serde_json::json!({
                            "workflow_id": workflow_id,
                            "step_index": idx,
                            "tool": step.tool,
                        }),
                        created_at: completed_at,
                    });
                }
                Err(e) => {
                    failed_steps = failed_steps.saturating_add(1);
                    st.status = "failed".to_string();
                    st.output = None;
                    st.error = Some(e.to_string());
                    state.status = "failed".to_string();
                    let _ = self.append_acp_event(AcpEventRecord {
                        id: uuid::Uuid::new_v4().to_string(),
                        session_id: Some(session_id.to_string()),
                        event_type: "workflow.step_failed".to_string(),
                        payload: serde_json::json!({
                            "workflow_id": workflow_id,
                            "step_index": idx,
                            "tool": step.tool,
                            "error": e.to_string(),
                        }),
                        created_at: completed_at,
                    });
                    state.updated_at = completed_at;
                    self.write_workflow_state(&state_path, &state)?;
                    if !continue_on_error {
                        break;
                    }
                }
            }

            state.updated_at = completed_at;
            self.write_workflow_state(&state_path, &state)?;
        }

        if state.status != "failed" {
            state.status = "completed".to_string();
        }
        state.completed_at = Some(Self::now_epoch_seconds());
        state.updated_at = state.completed_at.unwrap_or(state.updated_at);
        self.write_workflow_state(&state_path, &state)?;

        let _ = self.append_acp_event(AcpEventRecord {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: Some(session_id.to_string()),
            event_type: if state.status == "completed" {
                "workflow.completed".to_string()
            } else {
                "workflow.failed".to_string()
            },
            payload: serde_json::json!({
                "workflow_id": workflow_id,
                "status": state.status,
                "failed_steps": failed_steps,
            }),
            created_at: Self::now_epoch_seconds(),
        });

        Ok(serde_json::json!({
            "workflow_id": state.workflow_id,
            "name": state.name,
            "status": state.status,
            "failed_steps": failed_steps,
            "saved_to": state_path.display().to_string(),
        })
        .to_string())
    }

    fn write_workflow_state(
        &self,
        path: &std::path::Path,
        state: &WorkflowRunStateRecord,
    ) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create workflow dir {}", parent.display()))?;
        }
        let raw = serde_json::to_string_pretty(state).context("serialize workflow state")?;
        std::fs::write(path, raw)
            .with_context(|| format!("write workflow state {}", path.display()))
    }

    fn agents_index(&self) -> anyhow::Result<Vec<String>> {
        let raw = self
            .memory
            .kv_get("rexos.agents.index")
            .context("kv_get rexos.agents.index")?
            .unwrap_or_else(|| "[]".to_string());
        let ids: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();
        Ok(ids)
    }

    fn put_agents_index(&self, ids: &[String]) -> anyhow::Result<()> {
        let raw = serde_json::to_string(ids).context("serialize agents index")?;
        self.memory
            .kv_set("rexos.agents.index", &raw)
            .context("kv_set rexos.agents.index")?;
        Ok(())
    }

    fn agent_key(agent_id: &str) -> String {
        format!("rexos.agents.{agent_id}")
    }

    fn get_agent(&self, agent_id: &str) -> anyhow::Result<Option<AgentRecord>> {
        let raw = self
            .memory
            .kv_get(&Self::agent_key(agent_id))
            .with_context(|| format!("kv_get agent {agent_id}"))?;
        let Some(raw) = raw else { return Ok(None) };
        let record: AgentRecord =
            serde_json::from_str(&raw).with_context(|| format!("parse agent {agent_id}"))?;
        Ok(Some(record))
    }

    fn put_agent(&self, record: &AgentRecord) -> anyhow::Result<()> {
        let raw = serde_json::to_string(record).context("serialize agent record")?;
        self.memory
            .kv_set(&Self::agent_key(&record.id), &raw)
            .with_context(|| format!("kv_set agent {}", record.id))?;
        Ok(())
    }

    fn agent_spawn(&self, args: AgentSpawnToolArgs) -> anyhow::Result<String> {
        let mut name = args.name;
        let mut system_prompt = args.system_prompt;

        if let Some(manifest_toml) = args.manifest_toml.as_deref() {
            if let Ok(v) = manifest_toml.parse::<toml::Value>() {
                if name.is_none() {
                    name = v
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
                if system_prompt.is_none() {
                    system_prompt = v
                        .get("model")
                        .and_then(|m| m.get("system_prompt"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
            }
        }

        let agent_id = args
            .agent_id
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        if let Some(existing) = self.get_agent(&agent_id)? {
            return Ok(serde_json::to_string(&existing).unwrap_or_else(|_| "ok".to_string()));
        }

        let record = AgentRecord {
            id: agent_id.clone(),
            name,
            system_prompt,
            status: AgentStatus::Running,
            created_at: Self::now_epoch_seconds(),
            killed_at: None,
        };

        self.put_agent(&record)?;

        let mut index = self.agents_index()?;
        if !index.iter().any(|id| id == &agent_id) {
            index.push(agent_id);
            self.put_agents_index(&index)?;
        }

        Ok(serde_json::to_string(&record).unwrap_or_else(|_| "ok".to_string()))
    }

    fn agent_list(&self) -> anyhow::Result<String> {
        let index = self.agents_index()?;
        let mut out = Vec::new();
        for id in index {
            if let Some(record) = self.get_agent(&id)? {
                out.push(record);
            }
        }
        Ok(serde_json::to_string(&out).context("serialize agent list")?)
    }

    fn agent_find(&self, query: &str) -> anyhow::Result<String> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Ok("[]".to_string());
        }

        let index = self.agents_index()?;
        let mut out = Vec::new();
        for id in index {
            let Some(record) = self.get_agent(&id)? else {
                continue;
            };
            let hay =
                format!("{} {}", record.id, record.name.clone().unwrap_or_default()).to_lowercase();
            if hay.contains(&q) {
                out.push(record);
            }
        }

        Ok(serde_json::to_string(&out).context("serialize agent find")?)
    }

    fn agent_kill(&self, agent_id: &str) -> anyhow::Result<String> {
        let Some(mut record) = self.get_agent(agent_id)? else {
            return Ok(format!("error: agent not found: {agent_id}"));
        };
        record.status = AgentStatus::Killed;
        record.killed_at = Some(Self::now_epoch_seconds());
        self.put_agent(&record)?;
        Ok("ok".to_string())
    }

    async fn agent_send(
        &self,
        workspace_root: PathBuf,
        kind: TaskKind,
        args: AgentSendToolArgs,
    ) -> anyhow::Result<String> {
        let Some(record) = self.get_agent(&args.agent_id)? else {
            return Ok(format!("error: agent not found: {}", args.agent_id));
        };
        if record.status == AgentStatus::Killed {
            return Ok(format!("error: agent is killed: {}", args.agent_id));
        }

        let current_depth = AGENT_CALL_DEPTH.try_with(|d| d.get()).unwrap_or(0);
        if current_depth >= MAX_AGENT_CALL_DEPTH {
            return Ok(format!(
                "error: agent call depth exceeded (max {MAX_AGENT_CALL_DEPTH})"
            ));
        }

        let agent_id = args.agent_id.clone();
        let message = args.message.clone();
        let sys = record.system_prompt.clone();

        let out = AGENT_CALL_DEPTH
            .scope(std::cell::Cell::new(current_depth + 1), async {
                Box::pin(self.run_session(
                    workspace_root,
                    &agent_id,
                    sys.as_deref(),
                    &message,
                    kind,
                ))
                .await
            })
            .await;

        match out {
            Ok(v) => Ok(v),
            Err(e) => Ok(format!("error: {e}")),
        }
    }

    fn hand_defs() -> Vec<HandDef> {
        vec![
            HandDef {
                id: "browser",
                name: "Browser",
                description: "A focused web-browsing helper (use browser_* tools).",
                system_prompt: "You are a focused browser assistant. Use browser_* tools to navigate, read pages, and summarize findings clearly. Be careful with SSRF protections and only browse relevant URLs.",
            },
            HandDef {
                id: "coder",
                name: "Coder",
                description: "A focused coding helper (use fs_* and shell).",
                system_prompt: "You are a focused coding assistant. Use fs_read/fs_write/apply_patch and shell to implement changes safely. Prefer small commits, run tests, and explain how to reproduce.",
            },
            HandDef {
                id: "researcher",
                name: "Researcher",
                description: "A focused research helper (use web_search/web_fetch).",
                system_prompt: "You are a focused research assistant. Use web_search and web_fetch to gather information, then summarize with clear attribution. Avoid speculation and keep outputs concise.",
            },
        ]
    }

    fn hands_instances_index(&self) -> anyhow::Result<Vec<String>> {
        let raw = self
            .memory
            .kv_get("rexos.hands.instances.index")
            .context("kv_get rexos.hands.instances.index")?
            .unwrap_or_else(|| "[]".to_string());
        Ok(serde_json::from_str(&raw).unwrap_or_default())
    }

    fn put_hands_instances_index(&self, ids: &[String]) -> anyhow::Result<()> {
        let raw = serde_json::to_string(ids).context("serialize hands instances index")?;
        self.memory
            .kv_set("rexos.hands.instances.index", &raw)
            .context("kv_set rexos.hands.instances.index")?;
        Ok(())
    }

    fn hand_instance_key(instance_id: &str) -> String {
        format!("rexos.hands.instances.{instance_id}")
    }

    fn get_hand_instance(&self, instance_id: &str) -> anyhow::Result<Option<HandInstanceRecord>> {
        let raw = self
            .memory
            .kv_get(&Self::hand_instance_key(instance_id))
            .with_context(|| format!("kv_get hand instance {instance_id}"))?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        let record: HandInstanceRecord = serde_json::from_str(&raw)
            .with_context(|| format!("parse hand instance {instance_id}"))?;
        Ok(Some(record))
    }

    fn put_hand_instance(&self, record: &HandInstanceRecord) -> anyhow::Result<()> {
        let raw = serde_json::to_string(record).context("serialize hand instance record")?;
        self.memory
            .kv_set(&Self::hand_instance_key(&record.instance_id), &raw)
            .with_context(|| format!("kv_set hand instance {}", record.instance_id))?;
        Ok(())
    }

    fn hand_list(&self) -> anyhow::Result<String> {
        let defs = Self::hand_defs();
        let index = self.hands_instances_index()?;

        let mut instances = Vec::new();
        for id in index {
            if let Some(record) = self.get_hand_instance(&id)? {
                instances.push(record);
            }
        }

        let out: Vec<serde_json::Value> = defs
            .into_iter()
            .map(|d| {
                let active = instances
                    .iter()
                    .filter(|r| r.hand_id == d.id && r.status == HandInstanceStatus::Active)
                    .max_by_key(|r| r.created_at);

                serde_json::json!({
                    "id": d.id,
                    "name": d.name,
                    "description": d.description,
                    "status": if active.is_some() { "active" } else { "available" },
                    "instance_id": active.as_ref().map(|r| r.instance_id.clone()),
                    "agent_id": active.as_ref().map(|r| r.agent_id.clone()),
                })
            })
            .collect();

        Ok(serde_json::to_string(&out).context("serialize hand_list")?)
    }

    fn hand_activate(&self, args: HandActivateToolArgs) -> anyhow::Result<String> {
        let hand_id = args.hand_id.trim();
        if hand_id.is_empty() {
            bail!("hand_id is empty");
        }

        let def = Self::hand_defs()
            .into_iter()
            .find(|d| d.id == hand_id)
            .ok_or_else(|| anyhow::anyhow!("unknown hand_id: {hand_id}"))?;

        let instance_id = uuid::Uuid::new_v4().to_string();
        let agent_id = instance_id.clone();

        let mut system_prompt = def.system_prompt.to_string();
        if let Some(cfg) = args.config.as_ref() {
            system_prompt.push_str("\n\nHand config (JSON):\n");
            system_prompt.push_str(&serde_json::to_string_pretty(cfg).unwrap_or_default());
        }

        let _ = self.agent_spawn(AgentSpawnToolArgs {
            agent_id: Some(agent_id.clone()),
            name: Some(format!("hand:{hand_id}")),
            system_prompt: Some(system_prompt),
            manifest_toml: None,
        })?;

        let record = HandInstanceRecord {
            instance_id: instance_id.clone(),
            hand_id: hand_id.to_string(),
            agent_id: agent_id.clone(),
            status: HandInstanceStatus::Active,
            created_at: Self::now_epoch_seconds(),
            deactivated_at: None,
            config: args.config.unwrap_or(serde_json::Value::Null),
        };
        self.put_hand_instance(&record)?;

        let mut index = self.hands_instances_index()?;
        if !index.iter().any(|id| id == &instance_id) {
            index.push(instance_id.clone());
            self.put_hands_instances_index(&index)?;
        }

        Ok(serde_json::json!({
            "instance_id": instance_id,
            "hand_id": hand_id,
            "agent_id": agent_id,
            "status": "active",
        })
        .to_string())
    }

    fn hand_status(&self, hand_id: &str) -> anyhow::Result<String> {
        let hand_id = hand_id.trim();
        if hand_id.is_empty() {
            bail!("hand_id is empty");
        }

        let index = self.hands_instances_index()?;
        let mut active: Option<HandInstanceRecord> = None;

        for id in index {
            let Some(record) = self.get_hand_instance(&id)? else {
                continue;
            };
            if record.hand_id != hand_id {
                continue;
            }
            if record.status != HandInstanceStatus::Active {
                continue;
            }

            if active
                .as_ref()
                .map(|r| r.created_at <= record.created_at)
                .unwrap_or(true)
            {
                active = Some(record);
            }
        }

        let Some(active) = active else {
            return Ok(serde_json::json!({
                "hand_id": hand_id,
                "status": "inactive",
            })
            .to_string());
        };

        Ok(serde_json::to_string(&active).context("serialize hand_status")?)
    }

    fn hand_deactivate(&self, instance_id: &str) -> anyhow::Result<String> {
        let instance_id = instance_id.trim();
        if instance_id.is_empty() {
            bail!("instance_id is empty");
        }

        let Some(mut record) = self.get_hand_instance(instance_id)? else {
            return Ok(format!("error: hand instance not found: {instance_id}"));
        };

        if record.status == HandInstanceStatus::Deactivated {
            return Ok("ok".to_string());
        }

        record.status = HandInstanceStatus::Deactivated;
        record.deactivated_at = Some(Self::now_epoch_seconds());
        self.put_hand_instance(&record)?;

        let _ = self.agent_kill(&record.agent_id);
        Ok("ok".to_string())
    }

    fn tasks_index(&self) -> anyhow::Result<Vec<String>> {
        let raw = self
            .memory
            .kv_get("rexos.tasks.index")
            .context("kv_get rexos.tasks.index")?
            .unwrap_or_else(|| "[]".to_string());
        let ids: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();
        Ok(ids)
    }

    fn put_tasks_index(&self, ids: &[String]) -> anyhow::Result<()> {
        let raw = serde_json::to_string(ids).context("serialize tasks index")?;
        self.memory
            .kv_set("rexos.tasks.index", &raw)
            .context("kv_set rexos.tasks.index")?;
        Ok(())
    }

    fn task_key(task_id: &str) -> String {
        format!("rexos.tasks.{task_id}")
    }

    fn get_task(&self, task_id: &str) -> anyhow::Result<Option<TaskRecord>> {
        let raw = self
            .memory
            .kv_get(&Self::task_key(task_id))
            .with_context(|| format!("kv_get task {task_id}"))?;
        let Some(raw) = raw else { return Ok(None) };
        let record: TaskRecord =
            serde_json::from_str(&raw).with_context(|| format!("parse task {task_id}"))?;
        Ok(Some(record))
    }

    fn put_task(&self, record: &TaskRecord) -> anyhow::Result<()> {
        let raw = serde_json::to_string(record).context("serialize task record")?;
        self.memory
            .kv_set(&Self::task_key(&record.id), &raw)
            .with_context(|| format!("kv_set task {}", record.id))?;
        Ok(())
    }

    fn task_post(&self, args: TaskPostToolArgs) -> anyhow::Result<String> {
        let task_id = args
            .task_id
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        if let Some(existing) = self.get_task(&task_id)? {
            return Ok(serde_json::to_string(&existing).unwrap_or_else(|_| "ok".to_string()));
        }

        let record = TaskRecord {
            id: task_id.clone(),
            title: args.title,
            description: args.description,
            assigned_to: args.assigned_to,
            status: TaskStatus::Pending,
            claimed_by: None,
            result: None,
            created_at: Self::now_epoch_seconds(),
            claimed_at: None,
            completed_at: None,
        };

        self.put_task(&record)?;
        let mut index = self.tasks_index()?;
        if !index.iter().any(|id| id == &task_id) {
            index.push(task_id);
            self.put_tasks_index(&index)?;
        }

        Ok(serde_json::to_string(&record).unwrap_or_else(|_| "ok".to_string()))
    }

    fn task_list(&self, status: Option<&str>) -> anyhow::Result<String> {
        let wanted = status
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty());

        let index = self.tasks_index()?;
        let mut out = Vec::new();
        for id in index {
            let Some(record) = self.get_task(&id)? else {
                continue;
            };
            if let Some(wanted) = wanted.as_deref() {
                if record.status.as_str() != wanted {
                    continue;
                }
            }
            out.push(record);
        }

        Ok(serde_json::to_string(&out).context("serialize task_list")?)
    }

    fn task_claim(&self, agent_id: Option<&str>) -> anyhow::Result<String> {
        let agent_id = agent_id.map(|s| s.trim()).filter(|s| !s.is_empty());

        let index = self.tasks_index()?;
        for id in index {
            let Some(mut record) = self.get_task(&id)? else {
                continue;
            };
            if record.status != TaskStatus::Pending {
                continue;
            }
            if let Some(assigned) = record.assigned_to.as_deref() {
                let Some(agent_id) = agent_id else { continue };
                if assigned != agent_id {
                    continue;
                }
            }

            record.status = TaskStatus::Claimed;
            record.claimed_by = agent_id.map(|s| s.to_string());
            record.claimed_at = Some(Self::now_epoch_seconds());
            self.put_task(&record)?;
            return Ok(serde_json::to_string(&record).unwrap_or_else(|_| "ok".to_string()));
        }

        Ok("null".to_string())
    }

    fn task_complete(&self, task_id: &str, result: &str) -> anyhow::Result<String> {
        let Some(mut record) = self.get_task(task_id)? else {
            return Ok(format!("error: task not found: {task_id}"));
        };
        record.status = TaskStatus::Completed;
        record.result = Some(result.to_string());
        record.completed_at = Some(Self::now_epoch_seconds());
        self.put_task(&record)?;
        Ok("ok".to_string())
    }

    fn event_publish(&self, args: EventPublishToolArgs) -> anyhow::Result<String> {
        let key = "rexos.events";
        let raw = self
            .memory
            .kv_get(key)
            .context("kv_get rexos.events")?
            .unwrap_or_else(|| "[]".to_string());
        let mut events: Vec<EventRecord> = serde_json::from_str(&raw).unwrap_or_default();

        events.push(EventRecord {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: args.event_type,
            payload: args.payload.unwrap_or(serde_json::json!({})),
            created_at: Self::now_epoch_seconds(),
        });

        if events.len() > 200 {
            events.drain(0..(events.len() - 200));
        }

        let out = serde_json::to_string(&events).context("serialize rexos.events")?;
        self.memory
            .kv_set(key, &out)
            .context("kv_set rexos.events")?;
        Ok("ok".to_string())
    }

    fn schedules_get(&self) -> anyhow::Result<Vec<ScheduleRecord>> {
        let key = "rexos.schedules";
        let raw = self
            .memory
            .kv_get(key)
            .context("kv_get rexos.schedules")?
            .unwrap_or_else(|| "[]".to_string());
        let schedules: Vec<ScheduleRecord> = serde_json::from_str(&raw).unwrap_or_default();
        Ok(schedules)
    }

    fn schedules_set(&self, schedules: &[ScheduleRecord]) -> anyhow::Result<()> {
        let key = "rexos.schedules";
        let raw = serde_json::to_string(schedules).context("serialize rexos.schedules")?;
        self.memory
            .kv_set(key, &raw)
            .context("kv_set rexos.schedules")?;
        Ok(())
    }

    fn schedule_create(&self, args: ScheduleCreateToolArgs) -> anyhow::Result<String> {
        let id = args.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let mut schedules = self.schedules_get()?;
        if let Some(existing) = schedules.iter().find(|s| s.id == id) {
            return Ok(serde_json::to_string(existing).unwrap_or_else(|_| "ok".to_string()));
        }

        let agent_id = args.agent_id.or(args.agent);
        let record = ScheduleRecord {
            id: id.clone(),
            description: args.description,
            schedule: args.schedule,
            agent_id,
            created_at: Self::now_epoch_seconds(),
            enabled: args.enabled.unwrap_or(true),
        };

        schedules.push(record.clone());
        self.schedules_set(&schedules)?;

        Ok(serde_json::to_string(&record).unwrap_or_else(|_| "ok".to_string()))
    }

    fn schedule_list(&self) -> anyhow::Result<String> {
        let schedules = self.schedules_get()?;
        Ok(serde_json::to_string(&schedules).context("serialize schedule_list")?)
    }

    fn schedule_delete(&self, id: &str) -> anyhow::Result<String> {
        let mut schedules = self.schedules_get()?;
        let before = schedules.len();
        schedules.retain(|s| s.id != id);
        if schedules.len() == before {
            return Ok(format!("error: schedule not found: {id}"));
        }
        self.schedules_set(&schedules)?;
        Ok("ok".to_string())
    }

    fn cron_jobs_get(&self) -> anyhow::Result<Vec<CronJobRecord>> {
        let key = "rexos.cron.jobs";
        let raw = self
            .memory
            .kv_get(key)
            .context("kv_get rexos.cron.jobs")?
            .unwrap_or_else(|| "[]".to_string());
        let jobs: Vec<CronJobRecord> = serde_json::from_str(&raw).unwrap_or_default();
        Ok(jobs)
    }

    fn cron_jobs_set(&self, jobs: &[CronJobRecord]) -> anyhow::Result<()> {
        let key = "rexos.cron.jobs";
        let raw = serde_json::to_string(jobs).context("serialize rexos.cron.jobs")?;
        self.memory
            .kv_set(key, &raw)
            .context("kv_set rexos.cron.jobs")?;
        Ok(())
    }

    fn cron_create(&self, args: CronCreateToolArgs) -> anyhow::Result<String> {
        let job_id = args
            .job_id
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let mut jobs = self.cron_jobs_get()?;
        if let Some(existing) = jobs.iter().find(|j| j.job_id == job_id) {
            return Ok(serde_json::to_string(existing).unwrap_or_else(|_| "ok".to_string()));
        }

        let record = CronJobRecord {
            job_id: job_id.clone(),
            name: args.name,
            schedule: args.schedule,
            action: args.action,
            delivery: args.delivery,
            one_shot: args.one_shot.unwrap_or(false),
            created_at: Self::now_epoch_seconds(),
            enabled: args.enabled.unwrap_or(true),
        };

        jobs.push(record.clone());
        if jobs.len() > 200 {
            jobs.drain(0..(jobs.len() - 200));
        }
        self.cron_jobs_set(&jobs)?;

        Ok(serde_json::to_string(&record).unwrap_or_else(|_| "ok".to_string()))
    }

    fn cron_list(&self) -> anyhow::Result<String> {
        let jobs = self.cron_jobs_get()?;
        Ok(serde_json::to_string(&jobs).context("serialize cron_list")?)
    }

    fn cron_cancel(&self, job_id: &str) -> anyhow::Result<String> {
        let mut jobs = self.cron_jobs_get()?;
        let before = jobs.len();
        jobs.retain(|j| j.job_id != job_id);
        if jobs.len() == before {
            return Ok(format!("error: cron job not found: {job_id}"));
        }
        self.cron_jobs_set(&jobs)?;
        Ok("ok".to_string())
    }

    fn outbox_messages_get(&self) -> anyhow::Result<Vec<OutboxMessageRecord>> {
        let key = "rexos.outbox.messages";
        let raw = self
            .memory
            .kv_get(key)
            .context("kv_get rexos.outbox.messages")?
            .unwrap_or_else(|| "[]".to_string());
        let msgs: Vec<OutboxMessageRecord> = serde_json::from_str(&raw).unwrap_or_default();
        Ok(msgs)
    }

    fn outbox_messages_set(&self, msgs: &[OutboxMessageRecord]) -> anyhow::Result<()> {
        let key = "rexos.outbox.messages";
        let raw = serde_json::to_string(msgs).context("serialize rexos.outbox.messages")?;
        self.memory
            .kv_set(key, &raw)
            .context("kv_set rexos.outbox.messages")?;
        Ok(())
    }

    fn channel_send(
        &self,
        session_id: Option<&str>,
        args: ChannelSendToolArgs,
    ) -> anyhow::Result<String> {
        if args.channel.trim().is_empty() {
            return Ok("error: channel is empty".to_string());
        }
        if args.recipient.trim().is_empty() {
            return Ok("error: recipient is empty".to_string());
        }
        if args.message.trim().is_empty() {
            return Ok("error: message is empty".to_string());
        }

        match args.channel.as_str() {
            "console" | "webhook" => {}
            other => return Ok(format!("error: unknown channel: {other}")),
        }

        let now = Self::now_epoch_seconds();
        let record = OutboxMessageRecord {
            message_id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.map(|s| s.to_string()),
            channel: args.channel,
            recipient: args.recipient,
            subject: args.subject.filter(|s| !s.trim().is_empty()),
            message: args.message,
            status: OutboxStatus::Queued,
            attempts: 0,
            last_error: None,
            created_at: now,
            updated_at: now,
            sent_at: None,
        };

        let mut msgs = self.outbox_messages_get()?;
        msgs.push(record.clone());
        if msgs.len() > 500 {
            msgs.drain(0..(msgs.len() - 500));
        }
        self.outbox_messages_set(&msgs)?;

        Ok(serde_json::json!({
            "status": "queued",
            "message_id": record.message_id,
        })
        .to_string())
    }

    fn knowledge_entities_get(&self) -> anyhow::Result<Vec<KnowledgeEntityRecord>> {
        let key = "rexos.knowledge.entities";
        let raw = self
            .memory
            .kv_get(key)
            .context("kv_get rexos.knowledge.entities")?
            .unwrap_or_else(|| "[]".to_string());
        let entities: Vec<KnowledgeEntityRecord> = serde_json::from_str(&raw).unwrap_or_default();
        Ok(entities)
    }

    fn knowledge_entities_set(&self, entities: &[KnowledgeEntityRecord]) -> anyhow::Result<()> {
        let key = "rexos.knowledge.entities";
        let raw = serde_json::to_string(entities).context("serialize rexos.knowledge.entities")?;
        self.memory
            .kv_set(key, &raw)
            .context("kv_set rexos.knowledge.entities")?;
        Ok(())
    }

    fn knowledge_relations_get(&self) -> anyhow::Result<Vec<KnowledgeRelationRecord>> {
        let key = "rexos.knowledge.relations";
        let raw = self
            .memory
            .kv_get(key)
            .context("kv_get rexos.knowledge.relations")?
            .unwrap_or_else(|| "[]".to_string());
        let relations: Vec<KnowledgeRelationRecord> =
            serde_json::from_str(&raw).unwrap_or_default();
        Ok(relations)
    }

    fn knowledge_relations_set(&self, relations: &[KnowledgeRelationRecord]) -> anyhow::Result<()> {
        let key = "rexos.knowledge.relations";
        let raw =
            serde_json::to_string(relations).context("serialize rexos.knowledge.relations")?;
        self.memory
            .kv_set(key, &raw)
            .context("kv_set rexos.knowledge.relations")?;
        Ok(())
    }

    fn knowledge_add_entity(&self, args: KnowledgeAddEntityToolArgs) -> anyhow::Result<String> {
        let id = args.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let mut entities = self.knowledge_entities_get()?;
        if let Some(existing) = entities.iter().find(|e| e.id == id) {
            return Ok(serde_json::to_string(existing).unwrap_or_else(|_| "ok".to_string()));
        }

        let now = Self::now_epoch_seconds();
        let record = KnowledgeEntityRecord {
            id: id.clone(),
            name: args.name,
            entity_type: args.entity_type,
            properties: args.properties,
            created_at: now,
            updated_at: now,
        };

        entities.push(record.clone());
        self.knowledge_entities_set(&entities)?;

        Ok(serde_json::to_string(&record).unwrap_or_else(|_| "ok".to_string()))
    }

    fn knowledge_add_relation(&self, args: KnowledgeAddRelationToolArgs) -> anyhow::Result<String> {
        let id = args.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let mut relations = self.knowledge_relations_get()?;
        if let Some(existing) = relations.iter().find(|r| r.id == id) {
            return Ok(serde_json::to_string(existing).unwrap_or_else(|_| "ok".to_string()));
        }

        let record = KnowledgeRelationRecord {
            id: id.clone(),
            source: args.source,
            relation: args.relation,
            target: args.target,
            properties: args.properties,
            created_at: Self::now_epoch_seconds(),
        };

        relations.push(record.clone());
        self.knowledge_relations_set(&relations)?;

        Ok(serde_json::to_string(&record).unwrap_or_else(|_| "ok".to_string()))
    }

    fn knowledge_query(&self, query: &str) -> anyhow::Result<String> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Ok(r#"{"entities":[],"relations":[]}"#.to_string());
        }

        let entities = self.knowledge_entities_get()?;
        let relations = self.knowledge_relations_get()?;

        let matched_entities: Vec<KnowledgeEntityRecord> = entities
            .into_iter()
            .filter(|e| {
                e.id.to_lowercase().contains(&q)
                    || e.name.to_lowercase().contains(&q)
                    || e.entity_type.to_lowercase().contains(&q)
            })
            .collect();

        let matched_entity_ids: std::collections::HashSet<String> =
            matched_entities.iter().map(|e| e.id.clone()).collect();

        let matched_relations: Vec<KnowledgeRelationRecord> = relations
            .into_iter()
            .filter(|r| {
                r.id.to_lowercase().contains(&q)
                    || r.source.to_lowercase().contains(&q)
                    || r.target.to_lowercase().contains(&q)
                    || r.relation.to_lowercase().contains(&q)
                    || matched_entity_ids.contains(&r.source)
                    || matched_entity_ids.contains(&r.target)
            })
            .collect();

        Ok(serde_json::json!({
            "entities": matched_entities,
            "relations": matched_relations,
        })
        .to_string())
    }
}

fn workflow_state_path(workspace_root: &Path, workflow_id: &str) -> PathBuf {
    workspace_root
        .join(".loopforge")
        .join("workflows")
        .join(format!("{workflow_id}.json"))
}

fn is_runtime_managed_tool(name: &str) -> bool {
    matches!(
        name,
        "memory_store"
            | "memory_recall"
            | "agent_send"
            | "agent_spawn"
            | "agent_list"
            | "agent_kill"
            | "agent_find"
            | "hand_list"
            | "hand_activate"
            | "hand_status"
            | "hand_deactivate"
            | "task_post"
            | "task_claim"
            | "task_complete"
            | "task_list"
            | "event_publish"
            | "schedule_create"
            | "schedule_list"
            | "schedule_delete"
            | "knowledge_add_entity"
            | "knowledge_add_relation"
            | "knowledge_query"
            | "cron_create"
            | "cron_list"
            | "cron_cancel"
            | "channel_send"
            | "workflow_run"
    )
}
