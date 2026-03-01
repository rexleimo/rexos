use std::path::PathBuf;
use std::collections::HashMap;

use anyhow::{bail, Context};

use rexos_kernel::router::{ModelRouter, TaskKind};
use rexos_llm::driver::LlmDriver;
use rexos_llm::openai_compat::{ChatCompletionRequest, ChatMessage, Role, ToolCall, ToolFunction};
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
                    None => return Ok(assistant.content.unwrap_or_default()),
                },
            };

            for call in tool_calls {
                let sig = format!("{}|{}", call.function.name, call.function.arguments);
                let count = tool_call_counts.entry(sig.clone()).or_insert(0);
                *count += 1;
                if *count >= 3 {
                    bail!("tool loop detected: {sig}");
                }

                let args_json = normalize_tool_arguments(&call.function.name, &call.function.arguments);
                let output = tools
                    .call(&call.function.name, &args_json)
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

#[derive(Debug, serde::Deserialize)]
struct JsonToolCall {
    name: String,
    #[serde(alias = "args")]
    #[serde(default)]
    arguments: Option<serde_json::Value>,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

fn normalize_tool_arguments(tool_name: &str, raw_arguments_json: &str) -> String {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(raw_arguments_json) else {
        return raw_arguments_json.to_string();
    };

    let Some(obj) = v.as_object() else {
        return raw_arguments_json.to_string();
    };

    let matches_name = obj
        .get("function")
        .and_then(|v| v.as_str())
        .or_else(|| obj.get("name").and_then(|v| v.as_str()))
        .map(|name| name == tool_name)
        .unwrap_or(true);
    if !matches_name {
        return raw_arguments_json.to_string();
    }

    let Some(inner) = obj.get("arguments") else {
        return raw_arguments_json.to_string();
    };

    if let Some(s) = inner.as_str() {
        return s.to_string();
    }

    serde_json::to_string(inner).unwrap_or_else(|_| raw_arguments_json.to_string())
}

fn parse_tool_calls_from_json_content(content: &str) -> Option<Vec<ToolCall>> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(calls) = parse_json_tool_calls_from_value(value) {
            return Some(into_tool_calls(calls));
        }
    }

    let calls = extract_json_tool_calls_from_text(trimmed);
    if calls.is_empty() {
        return None;
    }
    Some(into_tool_calls(calls))
}

fn into_tool_calls(calls: Vec<JsonToolCall>) -> Vec<ToolCall> {
    let mut out = Vec::new();
    for (idx, call) in calls.into_iter().enumerate() {
        let args_value = call
            .arguments
            .unwrap_or_else(|| serde_json::Value::Object(call.extra));
        let args = if let Some(s) = args_value.as_str() {
            s.to_string()
        } else {
            serde_json::to_string(&args_value).unwrap_or_else(|_| "{}".to_string())
        };
        out.push(ToolCall {
            id: format!("call_json_{}", idx + 1),
            kind: "function".to_string(),
            function: ToolFunction {
                name: call.name,
                arguments: args,
            },
        });
    }
    out
}

fn parse_json_tool_calls_from_value(value: serde_json::Value) -> Option<Vec<JsonToolCall>> {
    if let Some(arr) = value.as_array() {
        let mut calls = Vec::new();
        for item in arr {
            calls.push(serde_json::from_value::<JsonToolCall>(item.clone()).ok()?);
        }
        return Some(calls);
    }

    serde_json::from_value::<JsonToolCall>(value).ok().map(|c| vec![c])
}

fn extract_json_tool_calls_from_text(content: &str) -> Vec<JsonToolCall> {
    let mut calls = Vec::new();
    for (start, _) in content.match_indices('{') {
        if calls.len() >= 16 {
            break;
        }
        let Some(end) = find_balanced_json_object_end(content, start) else {
            continue;
        };
        let slice = &content[start..end];
        let Ok(value) = serde_json::from_str::<serde_json::Value>(slice) else {
            continue;
        };
        let Some(mut parsed) = parse_json_tool_calls_from_value(value) else {
            continue;
        };
        calls.append(&mut parsed);
    }
    calls
}

fn find_balanced_json_object_end(s: &str, start: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    if start >= bytes.len() || bytes[start] != b'{' {
        return None;
    }

    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escape = false;

    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            if b == b'\\' {
                escape = true;
                continue;
            }
            if b == b'"' {
                in_string = false;
                continue;
            }
            continue;
        }

        match b {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
    }

    None
}
