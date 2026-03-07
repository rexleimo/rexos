use rexos_llm::openai_compat::{ToolDefinition, ToolFunctionDefinition};
use serde_json::json;

#[derive(Debug, serde::Deserialize)]
pub(crate) struct ShellArgs {
    pub(crate) command: String,
    #[serde(default)]
    pub(crate) timeout_ms: Option<u64>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct ShellExecArgs {
    pub(crate) command: String,
    #[serde(default)]
    pub(crate) timeout_seconds: Option<u64>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct DockerExecArgs {
    pub(crate) command: String,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct ProcessStartArgs {
    pub(crate) command: String,
    #[serde(default)]
    pub(crate) args: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct ProcessPollArgs {
    pub(crate) process_id: String,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct ProcessWriteArgs {
    pub(crate) process_id: String,
    pub(crate) data: String,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct ProcessKillArgs {
    pub(crate) process_id: String,
}

pub(crate) fn core_tool_defs() -> Vec<ToolDefinition> {
    vec![shell_def()]
}

pub(crate) fn compat_tool_defs() -> Vec<ToolDefinition> {
    let mut defs = Vec::new();

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "shell_exec".to_string(),
            description: "Execute a shell command and return its output.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "The command to execute" },
                    "timeout_seconds": { "type": "integer", "description": "Timeout in seconds (default: 30)" }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "docker_exec".to_string(),
            description: "Run a command inside a one-shot Docker container with the workspace mounted (disabled by default).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Command to execute inside the container (passed to `sh -lc`)." }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "process_start".to_string(),
            description: "Start a long-running process (REPL/server). Returns a process_id.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Executable to run (e.g. 'python', 'node', 'bash')." },
                    "args": { "type": "array", "items": { "type": "string" }, "description": "Optional command-line args." }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "process_poll".to_string(),
            description: "Drain buffered stdout/stderr from a running process (non-blocking).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "process_id": { "type": "string", "description": "Process id returned by process_start." }
                },
                "required": ["process_id"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "process_write".to_string(),
            description: "Write data to a running process's stdin (appends newline if missing).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "process_id": { "type": "string", "description": "Process id returned by process_start." },
                    "data": { "type": "string", "description": "Data to write to stdin." }
                },
                "required": ["process_id", "data"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "process_kill".to_string(),
            description: "Terminate a running process and clean up resources.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "process_id": { "type": "string", "description": "Process id returned by process_start." }
                },
                "required": ["process_id"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "process_list".to_string(),
            description: "List running processes started via process_start.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
    });

    defs
}

fn shell_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "shell".to_string(),
            description:
                "Run a shell command inside the workspace (bash on Unix, PowerShell on Windows)."
                    .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Command to run." },
                    "timeout_ms": { "type": "integer", "description": "Timeout in milliseconds (default 60000).", "minimum": 1 }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
        },
    }
}
