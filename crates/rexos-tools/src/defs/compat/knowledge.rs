use rexos_llm::openai_compat::{ToolDefinition, ToolFunctionDefinition};
use serde_json::json;

pub(crate) fn compat_tool_defs() -> Vec<ToolDefinition> {
    let mut defs = Vec::new();

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "knowledge_add_entity".to_string(),
            description: "Add an entity to the shared knowledge graph.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Optional stable entity id. If omitted, LoopForge generates one." },
                    "name": { "type": "string", "description": "Entity name." },
                    "entity_type": { "type": "string", "description": "Entity type (free-form string)." },
                    "properties": { "type": "object", "description": "Optional properties map.", "additionalProperties": true }
                },
                "required": ["name", "entity_type"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "knowledge_add_relation".to_string(),
            description: "Add a relation to the shared knowledge graph.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Optional stable relation id. If omitted, LoopForge generates one." },
                    "source": { "type": "string", "description": "Source entity id." },
                    "relation": { "type": "string", "description": "Relation type/name." },
                    "target": { "type": "string", "description": "Target entity id." },
                    "properties": { "type": "object", "description": "Optional properties map.", "additionalProperties": true }
                },
                "required": ["source", "relation", "target"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "knowledge_query".to_string(),
            description: "Query the shared knowledge graph.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Query string (substring match)." }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
        },
    });
    defs
}
