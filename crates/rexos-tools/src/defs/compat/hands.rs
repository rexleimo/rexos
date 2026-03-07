use rexos_llm::openai_compat::{ToolDefinition, ToolFunctionDefinition};
use serde_json::json;

pub(crate) fn compat_tool_defs() -> Vec<ToolDefinition> {
    let mut defs = Vec::new();

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "hand_list".to_string(),
            description:
                "List available Hands (curated autonomous packages) and their activation status."
                    .to_string(),
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
            name: "hand_activate".to_string(),
            description: "Activate a Hand (spawns a specialized agent instance).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "hand_id": { "type": "string", "description": "Hand id (e.g. 'browser', 'coder')." },
                    "config": { "type": "object", "description": "Optional hand configuration (stored and appended to the hand system prompt)." }
                },
                "required": ["hand_id"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "hand_status".to_string(),
            description: "Get status for a Hand by id.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "hand_id": { "type": "string", "description": "Hand id." }
                },
                "required": ["hand_id"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "hand_deactivate".to_string(),
            description: "Deactivate a running Hand instance.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "instance_id": { "type": "string", "description": "Hand instance id returned by hand_activate." }
                },
                "required": ["instance_id"],
                "additionalProperties": false
            }),
        },
    });
    defs
}
