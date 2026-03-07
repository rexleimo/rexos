use rexos_llm::openai_compat::{ToolDefinition, ToolFunctionDefinition};
use serde_json::json;

#[derive(Debug, serde::Deserialize)]
pub(crate) struct WebFetchArgs {
    pub(crate) url: String,
    #[serde(default)]
    pub(crate) timeout_ms: Option<u64>,
    #[serde(default)]
    pub(crate) max_bytes: Option<u64>,
    #[serde(default)]
    pub(crate) allow_private: bool,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct PdfArgs {
    pub(crate) path: String,
    #[serde(default)]
    pub(crate) pages: Option<String>,
    #[serde(default)]
    pub(crate) max_pages: Option<u64>,
    #[serde(default)]
    pub(crate) max_chars: Option<u64>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct WebSearchArgs {
    pub(crate) query: String,
    #[serde(default)]
    pub(crate) max_results: Option<u32>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct A2aDiscoverArgs {
    pub(crate) url: String,
    #[serde(default)]
    pub(crate) allow_private: bool,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct A2aSendArgs {
    #[serde(default)]
    pub(crate) agent_url: Option<String>,
    #[serde(default)]
    pub(crate) url: Option<String>,
    pub(crate) message: String,
    #[serde(default)]
    pub(crate) session_id: Option<String>,
    #[serde(default)]
    pub(crate) allow_private: bool,
}

pub(crate) fn core_tool_defs() -> Vec<ToolDefinition> {
    vec![web_fetch_def(), pdf_def(), pdf_extract_def()]
}

pub(crate) fn compat_tool_defs() -> Vec<ToolDefinition> {
    let mut defs = Vec::new();

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "web_search".to_string(),
            description: "Search the web and return a short list of results.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "The search query" },
                    "max_results": { "type": "integer", "description": "Maximum number of results to return (default: 5, max: 20)" }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "location_get".to_string(),
            description: "Get environment location metadata (os/arch/tz).".to_string(),
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
            name: "a2a_discover".to_string(),
            description: "Discover an external A2A agent by fetching its agent card at `/.well-known/agent.json`.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "Base URL of the remote agent (http/https)." },
                    "allow_private": { "type": "boolean", "description": "Allow loopback/private IPs (default false)." }
                },
                "required": ["url"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "a2a_send".to_string(),
            description: "Send a JSON-RPC `tasks/send` request to an external A2A agent endpoint.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "agent_url": { "type": "string", "description": "Full JSON-RPC endpoint URL (http/https)." },
                    "url": { "type": "string", "description": "Alias for agent_url." },
                    "message": { "type": "string", "description": "Message to send to the remote agent." },
                    "session_id": { "type": "string", "description": "Optional session id for continuity." },
                    "allow_private": { "type": "boolean", "description": "Allow loopback/private IPs (default false)." }
                },
                "required": ["message"],
                "additionalProperties": false
            }),
        },
    });

    defs
}

fn web_fetch_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "web_fetch".to_string(),
            description:
                "Fetch a URL via HTTP(S) and return a small response body (SSRF-protected)."
                    .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "HTTP(S) URL to fetch." },
                    "timeout_ms": { "type": "integer", "description": "Timeout in milliseconds (default 20000).", "minimum": 1 },
                    "max_bytes": { "type": "integer", "description": "Maximum bytes to return (default 200000).", "minimum": 1 },
                    "allow_private": { "type": "boolean", "description": "Allow fetching loopback/private IPs (default false)." }
                },
                "required": ["url"],
                "additionalProperties": false
            }),
        },
    }
}

fn pdf_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "pdf".to_string(),
            description: "Extract text from a PDF file in the workspace.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Workspace-relative PDF path." },
                    "pages": { "type": "string", "description": "Optional page selector (1-indexed). Examples: \"1\", \"1-3\", \"2,4-6\"." },
                    "max_pages": { "type": "integer", "description": "Max pages to extract (default 10, max 50).", "minimum": 1 },
                    "max_chars": { "type": "integer", "description": "Max characters to return (default 12000, max 50000).", "minimum": 1 }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
    }
}

fn pdf_extract_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "pdf_extract".to_string(),
            description: "Alias of `pdf` (extract text from a PDF in the workspace).".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Workspace-relative PDF path." },
                    "pages": { "type": "string", "description": "Optional page selector (1-indexed). Examples: \"1\", \"1-3\", \"2,4-6\"." },
                    "max_pages": { "type": "integer", "description": "Max pages to extract (default 10, max 50).", "minimum": 1 },
                    "max_chars": { "type": "integer", "description": "Max characters to return (default 12000, max 50000).", "minimum": 1 }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
    }
}
