use std::path::PathBuf;
use std::collections::HashMap;

use anyhow::{bail, Context};

use rexos_kernel::router::{ModelRouter, TaskKind};
use rexos_llm::driver::LlmDriver;
use rexos_llm::openai_compat::{ChatCompletionRequest, ChatMessage, Role};
use rexos_llm::registry::LlmRegistry;
use rexos_memory::MemoryStore;
use rexos_tools::Toolset;

#[derive(Debug)]
pub struct AgentRuntime {
    memory: MemoryStore,
    llms: LlmRegistry,
    router: ModelRouter,
}

impl AgentRuntime {
    pub fn new(memory: MemoryStore, llms: LlmRegistry, router: ModelRouter) -> Self {
        Self { memory, llms, router }
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
        let provider = self.router.provider_for(kind);
        let model = self.resolve_model(provider, kind)?;

        let driver = self
            .llms
            .driver(provider)
            .ok_or_else(|| anyhow::anyhow!("unknown provider: {provider}"))?;

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
        let mut tool_call_counts: HashMap<String, u32> = HashMap::new();
        for _ in 0..8usize {
            let req = ChatCompletionRequest {
                model: model.clone(),
                messages: messages.clone(),
                tools: tool_defs.clone(),
                temperature: None,
            };

            let assistant = self
                .driver_chat(&*driver, req)
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
                let sig = format!("{}|{}", call.function.name, call.function.arguments);
                let count = tool_call_counts.entry(sig.clone()).or_insert(0);
                *count += 1;
                if *count >= 3 {
                    bail!("tool loop detected: {sig}");
                }

                let output = tools
                    .call(&call.function.name, &call.function.arguments)
                    .await
                    .with_context(|| format!("tool {}", call.function.name))?;

                let tool_msg = ChatMessage {
                    role: Role::Tool,
                    content: Some(output),
                    name: Some(call.function.name),
                    tool_call_id: Some(call.id),
                    tool_calls: None,
                };
                self.memory.append_chat_message(session_id, &tool_msg)?;
                messages.push(tool_msg);
            }
        }

        bail!("max iterations exceeded")
    }

    fn resolve_model(&self, provider: &str, kind: TaskKind) -> anyhow::Result<String> {
        let configured = self.router.model_for(kind).trim();
        if configured.is_empty() || configured.eq_ignore_ascii_case("default") {
            let model = self
                .llms
                .default_model(provider)
                .ok_or_else(|| anyhow::anyhow!("provider missing default_model: {provider}"))?;
            Ok(model.to_string())
        } else {
            Ok(configured.to_string())
        }
    }

    async fn driver_chat(
        &self,
        driver: &(dyn LlmDriver),
        req: ChatCompletionRequest,
    ) -> anyhow::Result<ChatMessage> {
        driver.chat(req).await
    }
}
