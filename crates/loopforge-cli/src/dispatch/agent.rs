use crate::{cli::AgentCommand, runtime_env};

pub(super) async fn run(command: AgentCommand) -> anyhow::Result<()> {
    match command {
        AgentCommand::Run {
            workspace,
            mcp_config,
            prompt,
            system,
            session,
            kind,
            allowed_tools,
        } => {
            let (_paths, agent) = runtime_env::load_agent_runtime()?;

            let session_id = match session {
                Some(id) => id,
                None => rexos::harness::resolve_session_id(&workspace)?,
            };
            let allowed_tools = if allowed_tools.is_empty() {
                None
            } else {
                Some(allowed_tools)
            };
            let mcp_config_json = match mcp_config.as_ref() {
                Some(path) => {
                    let raw = std::fs::read_to_string(path)?;
                    let json: serde_json::Value =
                        serde_json::from_str(&raw).map_err(|err| anyhow::anyhow!("{err}"))?;
                    Some(serde_json::to_string(&json)?)
                }
                None => None,
            };
            agent.configure_session_tooling(&session_id, allowed_tools, mcp_config_json)?;
            let out = agent
                .run_session(
                    workspace,
                    &session_id,
                    system.as_deref(),
                    &prompt,
                    kind.into(),
                )
                .await?;
            println!("{out}");
            eprintln!("[loopforge] session_id={session_id}");
            Ok(())
        }
    }
}
