use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use anyhow::{bail, Context};
use rexos_kernel::router::TaskKind;
use rexos_llm::openai_compat::ChatCompletionRequest;
use rexos_tools::Toolset;

use super::history::initialize_session_messages;
use crate::tool_calls::parse_tool_calls_from_json_content;
use crate::AgentRuntime;

impl AgentRuntime {
    pub async fn run_session(
        &self,
        workspace_root: PathBuf,
        session_id: &str,
        system_prompt: Option<&str>,
        user_prompt: &str,
        kind: TaskKind,
    ) -> anyhow::Result<String> {
        let mut policy = self.load_session_policy_snapshot(session_id)?;
        let allowed_lookup: Option<HashSet<String>> = policy
            .allowed_tools
            .as_ref()
            .map(|tools| tools.iter().cloned().collect());
        let allowed_tools = policy.allowed_tools.take();
        let tools = Toolset::new_with_allowed_tools_security_and_mcp_config(
            workspace_root.clone(),
            allowed_tools,
            self.security.clone(),
            policy.mcp_config_json.as_deref(),
        )
        .await?;
        let provider = self.router.provider_for(kind);
        let model = self.resolve_model(provider, kind)?;

        let driver = self
            .llms
            .driver(provider)
            .ok_or_else(|| anyhow::anyhow!("unknown provider: {provider}"))?;

        let mut messages =
            initialize_session_messages(self, session_id, system_prompt, user_prompt)?;
        self.append_session_started_event(session_id, kind, user_prompt);

        let tool_defs = tools.definitions();
        let mut tool_call_counts: HashMap<String, u32> = HashMap::new();
        for _ in 0..8usize {
            let req = ChatCompletionRequest {
                model: model.clone(),
                messages: messages.clone(),
                tools: tool_defs.clone(),
                temperature: Some(0.0),
            };

            let assistant = self
                .driver_chat(&*driver, req)
                .await
                .context("llm chat completion")?;

            self.memory.append_chat_message(session_id, &assistant)?;
            messages.push(assistant.clone());

            let tool_calls = match assistant.tool_calls.clone() {
                Some(calls) if !calls.is_empty() => calls,
                _ => match assistant
                    .content
                    .as_deref()
                    .and_then(parse_tool_calls_from_json_content)
                {
                    Some(calls) => calls,
                    None => {
                        let out = assistant.content.unwrap_or_default();
                        self.append_session_completed_event(session_id, &out);
                        return Ok(out);
                    }
                },
            };

            for call in tool_calls {
                let sig = format!("{}|{}", call.function.name, call.function.arguments);
                let count = tool_call_counts.entry(sig.clone()).or_insert(0);
                *count += 1;
                if *count >= 3 {
                    bail!("tool loop detected: {sig}");
                }

                let tool_msg = self
                    .process_tool_call(
                        &workspace_root,
                        session_id,
                        kind,
                        allowed_lookup.as_ref(),
                        &tools,
                        call,
                    )
                    .await?;
                self.memory.append_chat_message(session_id, &tool_msg)?;
                messages.push(tool_msg);
            }
        }

        self.append_session_failed_event(session_id);
        bail!("max iterations exceeded")
    }
}
