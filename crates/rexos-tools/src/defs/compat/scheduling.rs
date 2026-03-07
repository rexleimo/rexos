use rexos_llm::openai_compat::{ToolDefinition, ToolFunctionDefinition};
use serde_json::json;

pub(crate) fn compat_tool_defs() -> Vec<ToolDefinition> {
    let mut defs = Vec::new();

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "schedule_create".to_string(),
            description: "Create a schedule entry (persisted).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Optional stable schedule id. If omitted, LoopForge generates one." },
                    "description": { "type": "string", "description": "Human-readable description." },
                    "schedule": { "type": "string", "description": "Schedule expression (stored as-is)." },
                    "agent_id": { "type": "string", "description": "Optional agent id to associate with this schedule." },
                    "agent": { "type": "string", "description": "Alias of agent_id (optional)." },
                    "enabled": { "type": "boolean", "description": "Whether this schedule is enabled (default: true)." }
                },
                "required": ["description", "schedule"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "schedule_list".to_string(),
            description: "List schedule entries.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "schedule_delete".to_string(),
            description: "Delete a schedule entry by id.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Schedule id." }
                },
                "required": ["id"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "cron_create".to_string(),
            description: "Create a cron/scheduled job record (persisted).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "job_id": { "type": "string", "description": "Optional stable job id. If omitted, LoopForge generates one." },
                    "name": { "type": "string", "description": "Job name." },
                    "schedule": { "type": "object", "description": "Schedule payload (stored as-is)." },
                    "action": { "type": "object", "description": "Action payload (stored as-is)." },
                    "delivery": { "type": "object", "description": "Optional delivery payload (stored as-is)." },
                    "one_shot": { "type": "boolean", "description": "If true, job should be considered one-shot (stored)." },
                    "enabled": { "type": "boolean", "description": "Whether this job is enabled (default: true)." }
                },
                "required": ["name", "schedule", "action"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "cron_list".to_string(),
            description: "List cron/scheduled job records.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "cron_cancel".to_string(),
            description: "Cancel a cron/scheduled job record by id.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "job_id": { "type": "string", "description": "Job id." }
                },
                "required": ["job_id"],
                "additionalProperties": false
            }),
        },
    });
    defs
}
