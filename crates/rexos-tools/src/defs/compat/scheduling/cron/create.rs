use rexos_llm::openai_compat::ToolDefinition;
use serde_json::json;

use super::super::shared::function_def;

pub(super) fn tool_def() -> ToolDefinition {
    function_def(
        "cron_create",
        "Create a cron/scheduled job record (persisted).",
        json!({
            "type": "object",
            "properties": {
                "job_id": { "type": "string", "description": "Optional stable job id. If omitted, LoopForge generates one." },
                "name": { "type": "string", "description": "Job name." },
                "schedule": { "type": "object", "description": "Schedule payload (stored). Built-in runner supports `{kind: \"every\", every_secs}` and `{kind: \"at\", at_epoch_seconds}`." },
                "action": { "type": "object", "description": "Action payload (stored). Built-in runner supports `{kind: \"system_event\", ...}` and `{kind: \"channel_send\", ...}`." },
                "delivery": { "type": "object", "description": "Optional delivery payload (stored). For `{kind:\"channel_send\"}`, this is treated as ChannelSend args." },
                "one_shot": { "type": "boolean", "description": "If true, job should be considered one-shot (stored)." },
                "enabled": { "type": "boolean", "description": "Whether this job is enabled (default: true)." }
            },
            "required": ["name", "schedule", "action"],
            "additionalProperties": false
        }),
    )
}
