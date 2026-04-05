use anyhow::{anyhow, bail};

use crate::leak_guard::{LeakGuardAudit, LeakGuardVerdict};
use crate::records::{AcpEventRecord, ToolAuditRecord};
use crate::tool_calls::truncate_tool_result_with_flag;
use crate::{tool_event_payload, AgentRuntime, MAX_TOOL_RESULT_CHARS};

pub(super) fn finalize_tool_output(
    runtime: &AgentRuntime,
    session_id: &str,
    tool_name: &str,
    duration_ms: u64,
    output_result: anyhow::Result<String>,
) -> anyhow::Result<String> {
    let (output, leak_guard) = match output_result {
        Ok(output) => match runtime.leak_guard.inspect_tool_output(output) {
            LeakGuardVerdict::Allowed { content, audit } => (content, audit),
            LeakGuardVerdict::Blocked { error, audit } => {
                let _ = runtime.append_acp_event(AcpEventRecord {
                    id: uuid::Uuid::new_v4().to_string(),
                    session_id: Some(session_id.to_string()),
                    event_type: "tool.blocked".to_string(),
                    payload: tool_event_payload(
                        tool_name,
                        None,
                        Some(&error),
                        Some("leak_guard"),
                        Some(&audit),
                    ),
                    created_at: AgentRuntime::now_epoch_seconds(),
                });
                let _ = runtime.append_tool_audit(ToolAuditRecord {
                    session_id: session_id.to_string(),
                    tool_name: tool_name.to_string(),
                    success: false,
                    duration_ms,
                    truncated: false,
                    error: Some(error.clone()),
                    leak_guard: Some(audit),
                    created_at: AgentRuntime::now_epoch_seconds(),
                });
                bail!(error);
            }
        },
        Err(err) => {
            let err_text = err
                .chain()
                .map(|cause| cause.to_string())
                .collect::<Vec<_>>()
                .join(": ");
            let (safe_error, leak_guard): (String, Option<LeakGuardAudit>) =
                match runtime.leak_guard.inspect_tool_output(err_text) {
                    LeakGuardVerdict::Allowed { content, audit } => (content, audit),
                    LeakGuardVerdict::Blocked { error, audit } => (error, Some(audit)),
                };
            let _ = runtime.append_acp_event(AcpEventRecord {
                id: uuid::Uuid::new_v4().to_string(),
                session_id: Some(session_id.to_string()),
                event_type: "tool.failed".to_string(),
                payload: tool_event_payload(
                    tool_name,
                    None,
                    Some(&safe_error),
                    None,
                    leak_guard.as_ref(),
                ),
                created_at: AgentRuntime::now_epoch_seconds(),
            });
            let _ = runtime.append_tool_audit(ToolAuditRecord {
                session_id: session_id.to_string(),
                tool_name: tool_name.to_string(),
                success: false,
                duration_ms,
                truncated: false,
                error: Some(safe_error.to_string()),
                leak_guard,
                created_at: AgentRuntime::now_epoch_seconds(),
            });
            return Err(anyhow!(safe_error.to_string()));
        }
    };

    let (output, truncated) = truncate_tool_result_with_flag(output, MAX_TOOL_RESULT_CHARS);
    let _ = runtime.append_tool_audit(ToolAuditRecord {
        session_id: session_id.to_string(),
        tool_name: tool_name.to_string(),
        success: true,
        duration_ms,
        truncated,
        error: None,
        leak_guard: leak_guard.clone(),
        created_at: AgentRuntime::now_epoch_seconds(),
    });
    let _ = runtime.append_acp_event(AcpEventRecord {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: Some(session_id.to_string()),
        event_type: "tool.succeeded".to_string(),
        payload: tool_event_payload(tool_name, Some(truncated), None, None, leak_guard.as_ref()),
        created_at: AgentRuntime::now_epoch_seconds(),
    });

    Ok(output)
}
