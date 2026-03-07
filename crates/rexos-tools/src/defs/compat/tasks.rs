use rexos_llm::openai_compat::{ToolDefinition, ToolFunctionDefinition};
use serde_json::json;

pub(crate) fn compat_tool_defs() -> Vec<ToolDefinition> {
    let mut defs = Vec::new();

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "task_post".to_string(),
            description: "Post a task into the shared task board.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "Optional stable task id. If omitted, LoopForge generates one." },
                    "title": { "type": "string", "description": "Short title." },
                    "description": { "type": "string", "description": "Task description." },
                    "assigned_to": { "type": "string", "description": "Optional assignee agent id." }
                },
                "required": ["title", "description"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "task_list".to_string(),
            description: "List tasks (optionally filtered by status).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "status": { "type": "string", "description": "Optional filter: pending | claimed | completed." }
                },
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "task_claim".to_string(),
            description: "Claim the next available pending task.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "agent_id": { "type": "string", "description": "Optional agent id claiming the task." }
                },
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "task_complete".to_string(),
            description: "Mark a task as completed.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "Task id." },
                    "result": { "type": "string", "description": "Completion result summary." }
                },
                "required": ["task_id", "result"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "event_publish".to_string(),
            description: "Publish an event into the shared event log.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "event_type": { "type": "string", "description": "Event type/name." },
                    "payload": { "type": "object", "description": "Optional event payload." }
                },
                "required": ["event_type"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "channel_send".to_string(),
            description:
                "Enqueue an outbound message into the outbox (delivery happens via dispatcher)."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel": { "type": "string", "description": "Channel adapter name (console, webhook)." },
                    "recipient": { "type": "string", "description": "Channel-specific recipient identifier." },
                    "subject": { "type": "string", "description": "Optional subject line (used by some channels)." },
                    "message": { "type": "string", "description": "Message body to send." }
                },
                "required": ["channel", "recipient", "message"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "workflow_run".to_string(),
            description:
                "Run a persisted multi-step workflow and save execution state under .loopforge/workflows/."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "workflow_id": { "type": "string", "description": "Optional stable workflow id. If omitted, LoopForge generates one." },
                    "name": { "type": "string", "description": "Optional workflow display name." },
                    "continue_on_error": { "type": "boolean", "description": "Whether to continue executing remaining steps after a failed step (default false)." },
                    "steps": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string", "description": "Optional step name." },
                                "tool": { "type": "string", "description": "Tool name to execute." },
                                "arguments": { "type": "object", "description": "Tool arguments JSON object." },
                                "approval_required": { "type": "boolean", "description": "Force approval gate for this step when approval mode is enabled." }
                            },
                            "required": ["tool"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["steps"],
                "additionalProperties": false
            }),
        },
    });
    defs
}
