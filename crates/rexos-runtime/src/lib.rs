use rexos_kernel::router::ModelRouter;
use rexos_kernel::security::SecurityConfig;
use rexos_llm::registry::LlmRegistry;
use rexos_memory::MemoryStore;

tokio::task_local! {
    static AGENT_CALL_DEPTH: std::cell::Cell<usize>;
}

const MAX_AGENT_CALL_DEPTH: usize = 4;
const MAX_TOOL_RESULT_CHARS: usize = 15_000;
const TOOL_AUDIT_KEY: &str = "rexos.audit.tool_calls";
const TOOL_AUDIT_MAX_RECORDS: usize = 2_000;
const SKILL_AUDIT_KEY: &str = "rexos.audit.skill_runs";
const SKILL_AUDIT_MAX_RECORDS: usize = 2_000;
pub(crate) const SESSION_ALLOWED_TOOLS_KEY_PREFIX: &str = "rexos.sessions.allowed_tools.";
pub(crate) const SESSION_ALLOWED_SKILLS_KEY_PREFIX: &str = "rexos.sessions.allowed_skills.";
pub(crate) const SESSION_SKILL_POLICY_KEY_PREFIX: &str = "rexos.sessions.skill_policy.";
const ACP_EVENTS_KEY: &str = "rexos.acp.events";
const ACP_EVENTS_MAX_RECORDS: usize = 5_000;
const ACP_CHECKPOINTS_KEY_PREFIX: &str = "rexos.acp.checkpoints.";

mod acp;
mod agents_hands;
mod approval;
mod knowledge;
mod leak_guard;
mod outbox;
mod records;
mod runtime_state;
mod runtime_utils;
mod scheduling;
mod session_runner;
mod session_skills;
mod tasks_events;
mod tool_calls;
mod workflow;

#[cfg(test)]
mod tests;

use leak_guard::LeakGuard;
pub use records::{AcpDeliveryCheckpointRecord, AcpEventRecord, SessionSkillPolicy};
pub(crate) use runtime_utils::{is_runtime_managed_tool, tool_event_payload, workflow_state_path};

#[derive(Debug)]
pub struct AgentRuntime {
    memory: MemoryStore,
    llms: LlmRegistry,
    router: ModelRouter,
    security: SecurityConfig,
    leak_guard: LeakGuard,
}

pub use outbox::{OutboxDispatcher, OutboxDrainSummary};
pub use scheduling::runner::{CronRunnerConfig, CronRunnerTickSummary};

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

    fn now_epoch_seconds() -> i64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}
