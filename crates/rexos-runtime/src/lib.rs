use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{bail, Context};

use rexos_kernel::router::{ModelRouter, TaskKind};
use rexos_llm::driver::LlmDriver;
use rexos_llm::openai_compat::{ChatCompletionRequest, ChatMessage, Role, ToolCall, ToolFunction};
use rexos_llm::registry::LlmRegistry;
use rexos_memory::MemoryStore;
use rexos_tools::Toolset;

tokio::task_local! {
    static AGENT_CALL_DEPTH: std::cell::Cell<usize>;
}

const MAX_AGENT_CALL_DEPTH: usize = 4;

#[derive(Debug)]
pub struct AgentRuntime {
    memory: MemoryStore,
    llms: LlmRegistry,
    router: ModelRouter,
}

impl AgentRuntime {
    pub fn new(memory: MemoryStore, llms: LlmRegistry, router: ModelRouter) -> Self {
        Self {
            memory,
            llms,
            router,
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
        let tools = Toolset::new(workspace_root.clone())?;
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
                    None => return Ok(assistant.content.unwrap_or_default()),
                },
            };

            for call in tool_calls {
                let sig = format!("{}|{}", call.function.name, call.function.arguments);
                let count = tool_call_counts.entry(sig.clone()).or_insert(0);
                *count += 1;
                if *count >= 3 {
                    bail!("tool loop detected: {sig}");
                }

                let args_json =
                    normalize_tool_arguments(&call.function.name, &call.function.arguments);
                let output = match call.function.name.as_str() {
                    "memory_store" => {
                        let args: MemoryStoreToolArgs =
                            serde_json::from_str(&args_json).context("parse memory_store args")?;
                        self.memory
                            .kv_set(&args.key, &args.value)
                            .context("memory_store kv_set")?;
                        "ok".to_string()
                    }
                    "memory_recall" => {
                        let args: MemoryRecallToolArgs =
                            serde_json::from_str(&args_json).context("parse memory_recall args")?;
                        self.memory
                            .kv_get(&args.key)
                            .context("memory_recall kv_get")?
                            .unwrap_or_default()
                    }
                    "agent_spawn" => {
                        let args: AgentSpawnToolArgs =
                            serde_json::from_str(&args_json).context("parse agent_spawn args")?;
                        self.agent_spawn(args).context("agent_spawn")?
                    }
                    "agent_list" => self.agent_list().context("agent_list")?,
                    "agent_find" => {
                        let args: AgentFindToolArgs =
                            serde_json::from_str(&args_json).context("parse agent_find args")?;
                        self.agent_find(&args.query).context("agent_find")?
                    }
                    "agent_kill" => {
                        let args: AgentKillToolArgs =
                            serde_json::from_str(&args_json).context("parse agent_kill args")?;
                        self.agent_kill(&args.agent_id).context("agent_kill")?
                    }
                    "agent_send" => {
                        let args: AgentSendToolArgs =
                            serde_json::from_str(&args_json).context("parse agent_send args")?;
                        self.agent_send(workspace_root.clone(), kind, args)
                            .await
                            .context("agent_send")?
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
                        let args: TaskClaimToolArgs =
                            serde_json::from_str(&args_json).context("parse task_claim args")?;
                        self.task_claim(args.agent_id.as_deref())
                            .context("task_claim")?
                    }
                    "task_complete" => {
                        let args: TaskCompleteToolArgs =
                            serde_json::from_str(&args_json).context("parse task_complete args")?;
                        self.task_complete(&args.task_id, &args.result)
                            .context("task_complete")?
                    }
                    "event_publish" => {
                        let args: EventPublishToolArgs =
                            serde_json::from_str(&args_json).context("parse event_publish args")?;
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
                        let args: CronCreateToolArgs =
                            serde_json::from_str(&args_json).context("parse cron_create args")?;
                        self.cron_create(args).context("cron_create")?
                    }
                    "cron_list" => self.cron_list().context("cron_list")?,
                    "cron_cancel" => {
                        let args: CronCancelToolArgs =
                            serde_json::from_str(&args_json).context("parse cron_cancel args")?;
                        self.cron_cancel(&args.job_id).context("cron_cancel")?
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
        driver: &(dyn LlmDriver),
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
        let job_id = args.job_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
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

#[derive(Debug, serde::Deserialize)]
struct MemoryStoreToolArgs {
    key: String,
    value: String,
}

#[derive(Debug, serde::Deserialize)]
struct MemoryRecallToolArgs {
    key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum AgentStatus {
    Running,
    Killed,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct AgentRecord {
    id: String,
    name: Option<String>,
    system_prompt: Option<String>,
    status: AgentStatus,
    created_at: i64,
    killed_at: Option<i64>,
}

#[derive(Debug, serde::Deserialize)]
struct AgentSpawnToolArgs {
    #[serde(default)]
    agent_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    system_prompt: Option<String>,
    #[serde(default)]
    manifest_toml: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct AgentSendToolArgs {
    agent_id: String,
    message: String,
}

#[derive(Debug, serde::Deserialize)]
struct AgentKillToolArgs {
    agent_id: String,
}

#[derive(Debug, serde::Deserialize)]
struct AgentFindToolArgs {
    query: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum TaskStatus {
    Pending,
    Claimed,
    Completed,
}

impl TaskStatus {
    fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Claimed => "claimed",
            TaskStatus::Completed => "completed",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TaskRecord {
    id: String,
    title: String,
    description: String,
    assigned_to: Option<String>,
    status: TaskStatus,
    claimed_by: Option<String>,
    result: Option<String>,
    created_at: i64,
    claimed_at: Option<i64>,
    completed_at: Option<i64>,
}

#[derive(Debug, serde::Deserialize)]
struct TaskPostToolArgs {
    #[serde(default)]
    task_id: Option<String>,
    title: String,
    description: String,
    #[serde(default)]
    assigned_to: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct TaskListToolArgs {
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct TaskClaimToolArgs {
    #[serde(default)]
    agent_id: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct TaskCompleteToolArgs {
    task_id: String,
    result: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct EventRecord {
    id: String,
    event_type: String,
    payload: serde_json::Value,
    created_at: i64,
}

#[derive(Debug, serde::Deserialize)]
struct EventPublishToolArgs {
    event_type: String,
    #[serde(default)]
    payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ScheduleRecord {
    id: String,
    description: String,
    schedule: String,
    agent_id: Option<String>,
    created_at: i64,
    enabled: bool,
}

#[derive(Debug, serde::Deserialize)]
struct ScheduleCreateToolArgs {
    #[serde(default)]
    id: Option<String>,
    description: String,
    schedule: String,
    #[serde(default)]
    agent_id: Option<String>,
    #[serde(default)]
    agent: Option<String>,
    #[serde(default)]
    enabled: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
struct ScheduleDeleteToolArgs {
    id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CronJobRecord {
    job_id: String,
    name: String,
    schedule: serde_json::Value,
    action: serde_json::Value,
    #[serde(default)]
    delivery: Option<serde_json::Value>,
    one_shot: bool,
    created_at: i64,
    enabled: bool,
}

#[derive(Debug, serde::Deserialize)]
struct CronCreateToolArgs {
    #[serde(default)]
    #[serde(alias = "id")]
    job_id: Option<String>,
    name: String,
    schedule: serde_json::Value,
    action: serde_json::Value,
    #[serde(default)]
    delivery: Option<serde_json::Value>,
    #[serde(default)]
    one_shot: Option<bool>,
    #[serde(default)]
    enabled: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
struct CronCancelToolArgs {
    #[serde(alias = "id")]
    job_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct KnowledgeEntityRecord {
    id: String,
    name: String,
    entity_type: String,
    properties: serde_json::Map<String, serde_json::Value>,
    created_at: i64,
    updated_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct KnowledgeRelationRecord {
    id: String,
    source: String,
    relation: String,
    target: String,
    properties: serde_json::Map<String, serde_json::Value>,
    created_at: i64,
}

#[derive(Debug, serde::Deserialize)]
struct KnowledgeAddEntityToolArgs {
    #[serde(default)]
    id: Option<String>,
    name: String,
    entity_type: String,
    #[serde(default)]
    properties: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
struct KnowledgeAddRelationToolArgs {
    #[serde(default)]
    id: Option<String>,
    source: String,
    relation: String,
    target: String,
    #[serde(default)]
    properties: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
struct KnowledgeQueryToolArgs {
    query: String,
}

#[derive(Debug, serde::Deserialize)]
struct JsonToolCall {
    name: String,
    #[serde(alias = "args")]
    #[serde(default)]
    arguments: Option<serde_json::Value>,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

fn normalize_tool_arguments(tool_name: &str, raw_arguments_json: &str) -> String {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(raw_arguments_json) else {
        return raw_arguments_json.to_string();
    };

    let Some(obj) = v.as_object() else {
        return raw_arguments_json.to_string();
    };

    let matches_name = obj
        .get("function")
        .and_then(|v| v.as_str())
        .or_else(|| obj.get("name").and_then(|v| v.as_str()))
        .map(|name| name == tool_name)
        .unwrap_or(true);
    if !matches_name {
        return raw_arguments_json.to_string();
    }

    let Some(inner) = obj.get("arguments") else {
        return raw_arguments_json.to_string();
    };

    if let Some(s) = inner.as_str() {
        return s.to_string();
    }

    serde_json::to_string(inner).unwrap_or_else(|_| raw_arguments_json.to_string())
}

fn parse_tool_calls_from_json_content(content: &str) -> Option<Vec<ToolCall>> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(calls) = parse_json_tool_calls_from_value(value) {
            return Some(into_tool_calls(calls));
        }
    }

    let calls = extract_json_tool_calls_from_text(trimmed);
    if calls.is_empty() {
        return None;
    }
    Some(into_tool_calls(calls))
}

fn into_tool_calls(calls: Vec<JsonToolCall>) -> Vec<ToolCall> {
    let mut out = Vec::new();
    for (idx, call) in calls.into_iter().enumerate() {
        let args_value = call
            .arguments
            .unwrap_or_else(|| serde_json::Value::Object(call.extra));
        let args = if let Some(s) = args_value.as_str() {
            s.to_string()
        } else {
            serde_json::to_string(&args_value).unwrap_or_else(|_| "{}".to_string())
        };
        out.push(ToolCall {
            id: format!("call_json_{}", idx + 1),
            kind: "function".to_string(),
            function: ToolFunction {
                name: call.name,
                arguments: args,
            },
        });
    }
    out
}

fn parse_json_tool_calls_from_value(value: serde_json::Value) -> Option<Vec<JsonToolCall>> {
    if let Some(arr) = value.as_array() {
        let mut calls = Vec::new();
        for item in arr {
            calls.push(serde_json::from_value::<JsonToolCall>(item.clone()).ok()?);
        }
        return Some(calls);
    }

    serde_json::from_value::<JsonToolCall>(value)
        .ok()
        .map(|c| vec![c])
}

fn extract_json_tool_calls_from_text(content: &str) -> Vec<JsonToolCall> {
    let mut calls = Vec::new();
    for (start, _) in content.match_indices('{') {
        if calls.len() >= 16 {
            break;
        }
        let Some(end) = find_balanced_json_object_end(content, start) else {
            continue;
        };
        let slice = &content[start..end];
        let Ok(value) = serde_json::from_str::<serde_json::Value>(slice) else {
            continue;
        };
        let Some(mut parsed) = parse_json_tool_calls_from_value(value) else {
            continue;
        };
        calls.append(&mut parsed);
    }
    calls
}

fn find_balanced_json_object_end(s: &str, start: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    if start >= bytes.len() || bytes[start] != b'{' {
        return None;
    }

    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escape = false;

    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            if b == b'\\' {
                escape = true;
                continue;
            }
            if b == b'"' {
                in_string = false;
                continue;
            }
            continue;
        }

        match b {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
    }

    None
}
