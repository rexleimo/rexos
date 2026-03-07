#[derive(Debug, serde::Deserialize)]
pub(crate) struct MemoryStoreToolArgs {
    pub(crate) key: String,
    pub(crate) value: String,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct MemoryRecallToolArgs {
    pub(crate) key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AgentStatus {
    Running,
    Killed,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct AgentRecord {
    pub(crate) id: String,
    pub(crate) name: Option<String>,
    pub(crate) system_prompt: Option<String>,
    pub(crate) status: AgentStatus,
    pub(crate) created_at: i64,
    pub(crate) killed_at: Option<i64>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct AgentSpawnToolArgs {
    #[serde(default)]
    pub(crate) agent_id: Option<String>,
    #[serde(default)]
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) system_prompt: Option<String>,
    #[serde(default)]
    pub(crate) manifest_toml: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct AgentSendToolArgs {
    pub(crate) agent_id: String,
    pub(crate) message: String,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct AgentKillToolArgs {
    pub(crate) agent_id: String,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct AgentFindToolArgs {
    pub(crate) query: String,
}

#[derive(Debug, Clone)]
pub(crate) struct HandDef {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) system_prompt: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HandInstanceStatus {
    Active,
    Deactivated,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct HandInstanceRecord {
    pub(crate) instance_id: String,
    pub(crate) hand_id: String,
    pub(crate) agent_id: String,
    pub(crate) status: HandInstanceStatus,
    pub(crate) created_at: i64,
    #[serde(default)]
    pub(crate) deactivated_at: Option<i64>,
    #[serde(default)]
    pub(crate) config: serde_json::Value,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct HandActivateToolArgs {
    pub(crate) hand_id: String,
    #[serde(default)]
    pub(crate) config: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct HandStatusToolArgs {
    pub(crate) hand_id: String,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct HandDeactivateToolArgs {
    pub(crate) instance_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskStatus {
    Pending,
    Claimed,
    Completed,
}

impl TaskStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Claimed => "claimed",
            TaskStatus::Completed => "completed",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct TaskRecord {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) assigned_to: Option<String>,
    pub(crate) status: TaskStatus,
    pub(crate) claimed_by: Option<String>,
    pub(crate) result: Option<String>,
    pub(crate) created_at: i64,
    pub(crate) claimed_at: Option<i64>,
    pub(crate) completed_at: Option<i64>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct TaskPostToolArgs {
    #[serde(default)]
    pub(crate) task_id: Option<String>,
    pub(crate) title: String,
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) assigned_to: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct TaskListToolArgs {
    #[serde(default)]
    pub(crate) status: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct TaskClaimToolArgs {
    #[serde(default)]
    pub(crate) agent_id: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct TaskCompleteToolArgs {
    pub(crate) task_id: String,
    pub(crate) result: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct EventRecord {
    pub(crate) id: String,
    pub(crate) event_type: String,
    pub(crate) payload: serde_json::Value,
    pub(crate) created_at: i64,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct EventPublishToolArgs {
    pub(crate) event_type: String,
    #[serde(default)]
    pub(crate) payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct ScheduleRecord {
    pub(crate) id: String,
    pub(crate) description: String,
    pub(crate) schedule: String,
    pub(crate) agent_id: Option<String>,
    pub(crate) created_at: i64,
    pub(crate) enabled: bool,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct ScheduleCreateToolArgs {
    #[serde(default)]
    pub(crate) id: Option<String>,
    pub(crate) description: String,
    pub(crate) schedule: String,
    #[serde(default)]
    pub(crate) agent_id: Option<String>,
    #[serde(default)]
    pub(crate) agent: Option<String>,
    #[serde(default)]
    pub(crate) enabled: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct ScheduleDeleteToolArgs {
    pub(crate) id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct CronJobRecord {
    pub(crate) job_id: String,
    pub(crate) name: String,
    pub(crate) schedule: serde_json::Value,
    pub(crate) action: serde_json::Value,
    #[serde(default)]
    pub(crate) delivery: Option<serde_json::Value>,
    pub(crate) one_shot: bool,
    pub(crate) created_at: i64,
    pub(crate) enabled: bool,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct CronCreateToolArgs {
    #[serde(default)]
    #[serde(alias = "id")]
    pub(crate) job_id: Option<String>,
    pub(crate) name: String,
    pub(crate) schedule: serde_json::Value,
    pub(crate) action: serde_json::Value,
    #[serde(default)]
    pub(crate) delivery: Option<serde_json::Value>,
    #[serde(default)]
    pub(crate) one_shot: Option<bool>,
    #[serde(default)]
    pub(crate) enabled: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct CronCancelToolArgs {
    #[serde(alias = "id")]
    pub(crate) job_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OutboxStatus {
    Queued,
    Sent,
    Failed,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct OutboxMessageRecord {
    pub(crate) message_id: String,
    #[serde(default)]
    pub(crate) session_id: Option<String>,
    pub(crate) channel: String,
    pub(crate) recipient: String,
    #[serde(default)]
    pub(crate) subject: Option<String>,
    pub(crate) message: String,
    pub(crate) status: OutboxStatus,
    pub(crate) attempts: u32,
    #[serde(default)]
    pub(crate) last_error: Option<String>,
    pub(crate) created_at: i64,
    pub(crate) updated_at: i64,
    #[serde(default)]
    pub(crate) sent_at: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct ToolAuditRecord {
    pub(crate) session_id: String,
    pub(crate) tool_name: String,
    pub(crate) success: bool,
    pub(crate) duration_ms: u64,
    pub(crate) truncated: bool,
    #[serde(default)]
    pub(crate) error: Option<String>,
    pub(crate) created_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct SessionSkillPolicy {
    #[serde(default)]
    pub allowlist: Vec<String>,
    #[serde(default)]
    pub require_approval: bool,
    #[serde(default = "default_true")]
    pub auto_approve_readonly: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct SkillAuditRecord {
    pub(crate) session_id: String,
    pub(crate) skill_name: String,
    pub(crate) success: bool,
    #[serde(default)]
    pub(crate) permissions: Vec<String>,
    #[serde(default)]
    pub(crate) error: Option<String>,
    pub(crate) created_at: i64,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct ChannelSendToolArgs {
    pub(crate) channel: String,
    pub(crate) recipient: String,
    #[serde(default)]
    pub(crate) subject: Option<String>,
    pub(crate) message: String,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct WorkflowRunToolArgs {
    #[serde(default)]
    pub(crate) workflow_id: Option<String>,
    #[serde(default)]
    pub(crate) name: Option<String>,
    pub(crate) steps: Vec<WorkflowStepToolArgs>,
    #[serde(default)]
    pub(crate) continue_on_error: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct WorkflowStepToolArgs {
    pub(crate) tool: String,
    #[serde(default)]
    pub(crate) arguments: serde_json::Value,
    #[serde(default)]
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) approval_required: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct WorkflowRunStateRecord {
    pub(crate) workflow_id: String,
    #[serde(default)]
    pub(crate) name: Option<String>,
    pub(crate) session_id: String,
    pub(crate) status: String,
    pub(crate) created_at: i64,
    pub(crate) updated_at: i64,
    #[serde(default)]
    pub(crate) completed_at: Option<i64>,
    pub(crate) steps: Vec<WorkflowStepStateRecord>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct WorkflowStepStateRecord {
    pub(crate) index: usize,
    #[serde(default)]
    pub(crate) name: Option<String>,
    pub(crate) tool: String,
    pub(crate) arguments: serde_json::Value,
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) output: Option<String>,
    #[serde(default)]
    pub(crate) error: Option<String>,
    #[serde(default)]
    pub(crate) started_at: Option<i64>,
    #[serde(default)]
    pub(crate) completed_at: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AcpEventRecord {
    pub id: String,
    #[serde(default)]
    pub session_id: Option<String>,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub created_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AcpDeliveryCheckpointRecord {
    pub channel: String,
    pub cursor: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct KnowledgeEntityRecord {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) entity_type: String,
    pub(crate) properties: serde_json::Map<String, serde_json::Value>,
    pub(crate) created_at: i64,
    pub(crate) updated_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct KnowledgeRelationRecord {
    pub(crate) id: String,
    pub(crate) source: String,
    pub(crate) relation: String,
    pub(crate) target: String,
    pub(crate) properties: serde_json::Map<String, serde_json::Value>,
    pub(crate) created_at: i64,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct KnowledgeAddEntityToolArgs {
    #[serde(default)]
    pub(crate) id: Option<String>,
    pub(crate) name: String,
    pub(crate) entity_type: String,
    #[serde(default)]
    pub(crate) properties: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct KnowledgeAddRelationToolArgs {
    #[serde(default)]
    pub(crate) id: Option<String>,
    pub(crate) source: String,
    pub(crate) relation: String,
    pub(crate) target: String,
    #[serde(default)]
    pub(crate) properties: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct KnowledgeQueryToolArgs {
    pub(crate) query: String,
}

fn default_true() -> bool {
    true
}
