use std::path::PathBuf;

use anyhow::{bail, Context};

use crate::llm::openai_compat::{ChatCompletionRequest, ChatMessage, OpenAiCompatibleClient, Role};
use crate::memory::MemoryStore;
use crate::router::{ModelRouter, TaskKind};
use crate::tools::Toolset;

#[derive(Debug)]
pub struct AgentRuntime {
    memory: MemoryStore,
    llm: OpenAiCompatibleClient,
    router: ModelRouter,
}

impl AgentRuntime {
    pub fn new(memory: MemoryStore, llm: OpenAiCompatibleClient, router: ModelRouter) -> Self {
        Self { memory, llm, router }
    }

    pub async fn run_session(
        &self,
        workspace_root: PathBuf,
        session_id: &str,
        system_prompt: Option<&str>,
        user_prompt: &str,
        kind: TaskKind,
    ) -> anyhow::Result<String> {
        let tools = Toolset::new(workspace_root)?;
        let model = self.router.model_for(kind).to_string();

        let mut messages = self
            .memory
            .list_chat_messages(session_id)
            .context("load session history")?;

        if let Some(system_prompt) = system_prompt {
            let has_system = messages.iter().any(|m| m.role == Role::System);
            if !has_system {
                let system_msg = ChatMessage {
                    role: Role::System,
                    content: Some(system_prompt.to_string()),
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                };
                self.memory.append_chat_message(session_id, &system_msg)?;
                messages.push(system_msg);
            }
        }

        let user_msg = ChatMessage {
            role: Role::User,
            content: Some(user_prompt.to_string()),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        };
        self.memory.append_chat_message(session_id, &user_msg)?;
        messages.push(user_msg);

        let tool_defs = tools.definitions();
        for _ in 0..8usize {
            let req = ChatCompletionRequest {
                model: model.clone(),
                messages: messages.clone(),
                tools: tool_defs.clone(),
                temperature: None,
            };

            let assistant = self
                .llm
                .chat_completions(req)
                .await
                .context("llm chat completion")?;

            self.memory.append_chat_message(session_id, &assistant)?;
            messages.push(assistant.clone());

            let tool_calls = match assistant.tool_calls.clone() {
                Some(calls) if !calls.is_empty() => calls,
                _ => {
                    return Ok(assistant.content.unwrap_or_default());
                }
            };

            for call in tool_calls {
                let output = tools
                    .call(&call.function.name, &call.function.arguments)
                    .await
                    .with_context(|| format!("tool {}", call.function.name))?;

                let tool_msg = ChatMessage {
                    role: Role::Tool,
                    content: Some(output),
                    name: None,
                    tool_call_id: Some(call.id),
                    tool_calls: None,
                };
                self.memory.append_chat_message(session_id, &tool_msg)?;
                messages.push(tool_msg);
            }
        }

        bail!("max iterations exceeded")
    }
}
