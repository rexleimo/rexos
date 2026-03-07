use rexos_llm::openai_compat::{ToolDefinition, ToolFunctionDefinition};
use serde_json::json;

pub(crate) fn compat_tool_defs() -> Vec<ToolDefinition> {
    let mut defs = Vec::new();

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "agent_spawn".to_string(),
            description: "Create an agent session record (persisted) and return its details."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "agent_id": { "type": "string", "description": "Optional stable agent id. If omitted, LoopForge generates one." },
                    "name": { "type": "string", "description": "Optional human-friendly name." },
                    "system_prompt": { "type": "string", "description": "Optional system prompt for the agent session." },
                    "manifest_toml": { "type": "string", "description": "Optional agent manifest (TOML). LoopForge will best-effort extract name + system prompt." }
                },
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "agent_list".to_string(),
            description: "List known agent sessions.".to_string(),
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
            name: "agent_find".to_string(),
            description: "Find agent sessions by id or name (substring match).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query (case-insensitive substring)." }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "agent_kill".to_string(),
            description: "Mark an agent session as killed.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "agent_id": { "type": "string", "description": "Target agent id." }
                },
                "required": ["agent_id"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "agent_send".to_string(),
            description: "Send a message to an agent session and return its response.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "agent_id": { "type": "string", "description": "Target agent id." },
                    "message": { "type": "string", "description": "Message to send." }
                },
                "required": ["agent_id", "message"],
                "additionalProperties": false
            }),
        },
    });
    defs
}
