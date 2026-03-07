use rexos_llm::openai_compat::{ToolDefinition, ToolFunctionDefinition};
use serde_json::json;

pub(crate) fn compat_tool_defs() -> Vec<ToolDefinition> {
    let mut defs = Vec::new();

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "memory_store".to_string(),
            description: "Persist a key/value pair to shared memory.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "The memory key" },
                    "value": { "type": "string", "description": "The value to store" }
                },
                "required": ["key", "value"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "memory_recall".to_string(),
            description: "Recall a value from shared memory.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "The memory key" }
                },
                "required": ["key"],
                "additionalProperties": false
            }),
        },
    });
    defs
}
