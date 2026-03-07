use rexos_llm::openai_compat::{ToolDefinition, ToolFunctionDefinition};
use serde_json::json;

#[derive(Debug, serde::Deserialize)]
pub(crate) struct FsReadArgs {
    pub(crate) path: String,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct FileReadArgs {
    pub(crate) path: String,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct FsWriteArgs {
    pub(crate) path: String,
    pub(crate) content: String,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct FileWriteArgs {
    pub(crate) path: String,
    pub(crate) content: String,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct FileListArgs {
    pub(crate) path: String,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct ApplyPatchArgs {
    pub(crate) patch: String,
}

pub(crate) fn core_tool_defs() -> Vec<ToolDefinition> {
    vec![fs_read_def(), fs_write_def()]
}

pub(crate) fn compat_tool_defs() -> Vec<ToolDefinition> {
    let mut defs = Vec::new();

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "file_read".to_string(),
            description: "Read the contents of a file. Paths are relative to the agent workspace."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "The file path to read" }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "file_write".to_string(),
            description: "Write content to a file. Paths are relative to the agent workspace."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "The file path to write to" },
                    "content": { "type": "string", "description": "The content to write" }
                },
                "required": ["path", "content"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "file_list".to_string(),
            description: "List files in a directory. Paths are relative to the agent workspace."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "The directory path to list" }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "apply_patch".to_string(),
            description: "Apply a multi-hunk diff patch to add, update, or delete files."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "patch": { "type": "string", "description": "Patch in *** Begin Patch / *** End Patch format." }
                },
                "required": ["patch"],
                "additionalProperties": false
            }),
        },
    });

    defs
}

fn fs_read_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "fs_read".to_string(),
            description: "Read a UTF-8 text file from the workspace.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path inside the workspace." }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
    }
}

fn fs_write_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "fs_write".to_string(),
            description: "Write a UTF-8 text file to the workspace (creates parent dirs)."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path inside the workspace." },
                    "content": { "type": "string", "description": "Full file contents to write." }
                },
                "required": ["path", "content"],
                "additionalProperties": false
            }),
        },
    }
}
