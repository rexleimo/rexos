mod config;
mod jsonrpc;
mod routing;
mod stdio;
mod transport;
mod types;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::Arc;

use anyhow::Context;
use rexos_llm::openai_compat::ToolDefinition;

pub(crate) use config::{McpServerConfig, McpServersConfig};
use stdio::StdioServer;
use transport::ConnectedMcp;

#[derive(Debug, Clone)]
pub(crate) struct McpHub {
    servers: BTreeMap<String, Arc<McpServer>>,
    tool_targets: HashMap<String, McpToolTarget>,
    tool_defs: Vec<ToolDefinition>,
}

#[derive(Debug)]
struct McpServer {
    #[allow(dead_code)]
    name: String,
    stdio: StdioServer,
}

#[derive(Debug, Clone)]
struct McpToolTarget {
    server: String,
    remote_name: String,
}

impl McpHub {
    pub(crate) async fn connect_from_json(
        config_json: &str,
        workspace_root: &Path,
    ) -> anyhow::Result<Self> {
        let cfg: McpServersConfig =
            serde_json::from_str(config_json).context("parse mcp servers config JSON")?;
        Self::connect(cfg, workspace_root).await
    }

    pub(crate) async fn connect(
        cfg: McpServersConfig,
        workspace_root: &Path,
    ) -> anyhow::Result<Self> {
        let ConnectedMcp {
            servers,
            tool_targets,
            tool_defs,
        } = transport::connect(cfg, workspace_root).await?;

        Ok(Self {
            servers,
            tool_targets,
            tool_defs,
        })
    }

    pub(crate) fn tool_definitions(&self) -> &[ToolDefinition] {
        &self.tool_defs
    }

    pub(crate) fn server_names(&self) -> Vec<String> {
        self.servers.keys().cloned().collect()
    }
}
