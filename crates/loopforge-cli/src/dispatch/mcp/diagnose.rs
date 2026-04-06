use std::path::PathBuf;

use anyhow::{anyhow, Context};
use serde_json::Value;

use super::super::mcp_sanitize::sanitize_mcp_config;
use super::json::{build_mcp_diagnose_json, normalize_json_string};
use super::tools::list_remote_tool_names;
use crate::runtime_env;

pub(super) struct McpDiagnoseData {
    pub(super) sanitized_config: Value,
    pub(super) servers_raw: String,
    pub(super) servers_json: Value,
    pub(super) tool_names: Vec<String>,
    pub(super) resources_json: Option<Value>,
    pub(super) prompts_json: Option<Value>,
}

pub(super) async fn run_diagnose(
    workspace: PathBuf,
    session: Option<String>,
    config: Option<PathBuf>,
    resources: bool,
    prompts: bool,
    json: bool,
) -> anyhow::Result<()> {
    let (_paths, agent) = runtime_env::load_agent_runtime()?;
    let (_paths, cfg) = runtime_env::load_runtime_config()?;

    std::fs::create_dir_all(&workspace)
        .with_context(|| format!("create workspace: {}", workspace.display()))?;

    let session_id = match session {
        Some(id) => id,
        None => rexos::harness::resolve_session_id(&workspace)?,
    };

    let raw_config = match config.as_ref() {
        Some(path) => {
            let raw = std::fs::read_to_string(path)
                .with_context(|| format!("read mcp config: {}", path.display()))?;
            normalize_json_string(&raw).context("normalize mcp config json")?
        }
        None => {
            let snapshot = agent
                .load_session_policy_snapshot(&session_id)
                .with_context(|| format!("load session policy snapshot: {session_id}"))?;
            snapshot
                .mcp_config_json
                .ok_or_else(|| anyhow!("no MCP config is set for session {session_id}"))?
        }
    };

    let data = collect_mcp_diagnose_data(
        workspace.clone(),
        &raw_config,
        cfg.security.clone(),
        resources,
        prompts,
    )
    .await?;

    if json {
        let out = build_mcp_diagnose_json(
            &workspace,
            &session_id,
            data.sanitized_config,
            data.servers_json,
            data.tool_names,
            data.resources_json,
            data.prompts_json,
        )?;
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    println!("MCP diagnose");
    println!();
    println!("workspace: {}", workspace.display());
    println!("session_id: {session_id}");
    if let Some(path) = config.as_ref() {
        println!("config_source: file {}", path.display());
    } else {
        println!("config_source: session");
    }
    if let Some(servers) = data
        .sanitized_config
        .get("servers")
        .and_then(|v| v.as_object())
        .map(|obj| obj.keys().cloned().collect::<Vec<_>>())
    {
        if !servers.is_empty() {
            println!("config_servers: {}", servers.join(", "));
        }
    }
    println!();
    println!("servers: {}", data.servers_raw.trim());
    println!("remote_tools: {} tool(s)", data.tool_names.len());
    for name in data.tool_names {
        println!("- {name}");
    }
    if let Some(v) = data.resources_json {
        println!();
        println!("resources_list: {}", serde_json::to_string_pretty(&v)?);
    }
    if let Some(v) = data.prompts_json {
        println!();
        println!("prompts_list: {}", serde_json::to_string_pretty(&v)?);
    }
    Ok(())
}

pub(super) async fn collect_mcp_diagnose_data(
    workspace: PathBuf,
    raw_config: &str,
    security: rexos::security::SecurityConfig,
    resources: bool,
    prompts: bool,
) -> anyhow::Result<McpDiagnoseData> {
    let parsed_config: Value = serde_json::from_str(raw_config).context("parse mcp config JSON")?;
    let sanitized_config = sanitize_mcp_config(&parsed_config);

    let mut tools = rexos::tools::Toolset::new_with_security_config(workspace, security)?;
    tools
        .enable_mcp_from_json(raw_config)
        .await
        .context("connect mcp servers")?;

    let servers_raw = tools.call("mcp_servers_list", r#"{}"#).await?;
    let servers_json: Value =
        serde_json::from_str(&servers_raw).context("decode mcp_servers_list output")?;
    let tool_names = list_remote_tool_names(&tools);

    let resources_json = if resources {
        let out = tools.call("mcp_resources_list", r#"{}"#).await?;
        Some(serde_json::from_str(&out).context("decode mcp_resources_list output")?)
    } else {
        None
    };

    let prompts_json = if prompts {
        let out = tools.call("mcp_prompts_list", r#"{}"#).await?;
        Some(serde_json::from_str(&out).context("decode mcp_prompts_list output")?)
    } else {
        None
    };

    Ok(McpDiagnoseData {
        sanitized_config,
        servers_raw,
        servers_json,
        tool_names,
        resources_json,
        prompts_json,
    })
}
