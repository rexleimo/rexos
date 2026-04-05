use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Context};
use rexos_llm::openai_compat::{ToolDefinition, ToolFunctionDefinition};

use super::config::McpServersConfig;
use super::jsonrpc::JsonRpcClient;
use super::stdio::spawn_stdio_server;
use super::types::{self, ToolsListResult};
use super::{McpServer, McpToolTarget};

pub(super) struct ConnectedMcp {
    pub(super) servers: BTreeMap<String, Arc<McpServer>>,
    pub(super) tool_targets: HashMap<String, McpToolTarget>,
    pub(super) tool_defs: Vec<ToolDefinition>,
}

pub(super) async fn connect(
    cfg: McpServersConfig,
    workspace_root: &Path,
) -> anyhow::Result<ConnectedMcp> {
    if cfg.servers.is_empty() {
        return Err(anyhow!("mcp config has no servers"));
    }

    let mut servers: BTreeMap<String, Arc<McpServer>> = BTreeMap::new();
    let mut tool_targets: HashMap<String, McpToolTarget> = HashMap::new();
    let mut tool_defs: Vec<ToolDefinition> = Vec::new();
    let mut used_names: HashSet<String> = HashSet::new();

    for (name, server_cfg) in &cfg.servers {
        let stdio = spawn_stdio_server(name, server_cfg, workspace_root).await?;
        if let Err(err) = initialize(&stdio.client)
            .await
            .with_context(|| format!("mcp initialize: {name}"))
        {
            return Err(append_stderr_tail_context(err, name, &stdio).await);
        }

        let tools = match list_all_tools(&stdio.client)
            .await
            .with_context(|| format!("mcp tools/list: {name}"))
        {
            Ok(tools) => tools,
            Err(err) => return Err(append_stderr_tail_context(err, name, &stdio).await),
        };

        let server = Arc::new(McpServer {
            name: name.clone(),
            stdio,
        });

        for tool in tools {
            let local = allocate_local_tool_name(name, &tool.name, &mut used_names);
            tool_targets.insert(
                local.clone(),
                McpToolTarget {
                    server: name.clone(),
                    remote_name: tool.name.clone(),
                },
            );

            tool_defs.push(ToolDefinition {
                kind: "function".to_string(),
                function: ToolFunctionDefinition {
                    name: local,
                    description: tool
                        .description
                        .unwrap_or_else(|| format!("MCP tool '{name}::{}'", tool.name)),
                    parameters: if tool.input_schema.is_null() {
                        serde_json::json!({ "type": "object" })
                    } else {
                        tool.input_schema
                    },
                },
            });
        }

        servers.insert(name.clone(), server);
    }

    Ok(ConnectedMcp {
        servers,
        tool_targets,
        tool_defs,
    })
}

async fn append_stderr_tail_context(
    err: anyhow::Error,
    name: &str,
    stdio: &super::stdio::StdioServer,
) -> anyhow::Error {
    let tail = stdio.stderr_tail().lock().await.clone();
    if tail.is_empty() {
        return err;
    }

    err.context(format!(
        "mcp server '{name}' stderr tail (last {} lines):\n{}",
        tail.len(),
        tail.join("\n")
    ))
}

async fn initialize(client: &JsonRpcClient) -> anyhow::Result<()> {
    // Try a small set of known protocol revisions (latest-first) for broad compatibility.
    const VERSIONS: [&str; 3] = ["2025-11-25", "2025-03-26", "2024-11-05"];

    let params_base = |protocol: &str| {
        serde_json::json!({
            "protocolVersion": protocol,
            "capabilities": {},
            "clientInfo": {
                "name": "loopforge",
                "version": env!("CARGO_PKG_VERSION"),
            }
        })
    };

    let mut last_err: Option<anyhow::Error> = None;
    for v in VERSIONS {
        match client.request("initialize", Some(params_base(v))).await {
            Ok(_) => {
                client.notify("initialized", None).await?;
                return Ok(());
            }
            Err(err) => {
                last_err = Some(err);
            }
        }
    }

    Err(last_err.unwrap_or_else(|| anyhow!("mcp initialize failed")))
}

async fn list_all_tools(client: &JsonRpcClient) -> anyhow::Result<Vec<types::McpTool>> {
    let mut cursor: Option<String> = None;
    let mut out: Vec<types::McpTool> = Vec::new();
    for _ in 0..32usize {
        let params = cursor
            .as_deref()
            .map(|cursor| serde_json::json!({ "cursor": cursor }));
        let value = client.request("tools/list", params).await?;
        let parsed: ToolsListResult =
            serde_json::from_value(value).context("decode tools/list result")?;
        out.extend(parsed.tools.into_iter());
        cursor = parsed.next_cursor;
        if cursor.as_deref().unwrap_or("").trim().is_empty() {
            break;
        }
    }
    Ok(out)
}

pub(super) fn allocate_local_tool_name(
    server: &str,
    tool: &str,
    used: &mut HashSet<String>,
) -> String {
    let server_part = sanitize_component(server);
    let tool_part = sanitize_component(tool);
    let mut candidate = format!("mcp_{server_part}__{tool_part}");

    if candidate.len() > 64 {
        let hash = short_hash(&candidate);
        let suffix = format!("_{hash:08x}");
        candidate.truncate(64usize.saturating_sub(suffix.len()));
        candidate.push_str(&suffix);
    }

    if used.insert(candidate.clone()) {
        return candidate;
    }

    let hash = short_hash(&format!("{server}\0{tool}"));
    let suffix = format!("_{hash:08x}");
    let mut out = candidate;
    if out.len() + suffix.len() > 64 {
        out.truncate(64usize.saturating_sub(suffix.len()));
    }
    out.push_str(&suffix);
    used.insert(out.clone());
    out
}

pub(super) fn sanitize_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        let c = if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            c.to_ascii_lowercase()
        } else {
            '_'
        };
        out.push(c);
    }
    out.trim_matches('_').to_string()
}

fn short_hash(value: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut h);
    h.finish()
}
