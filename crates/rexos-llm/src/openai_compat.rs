use std::time::Duration;

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: Role,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub kind: String,
    pub function: ToolFunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolFunctionDefinition {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tools: Vec<ToolDefinition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatCompletionResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Choice {
    pub index: usize,
    pub message: ChatMessage,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawChatCompletionResponse {
    choices: Vec<RawChoice>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawChoice {
    message: RawChatMessage,
}

#[derive(Debug, Clone, Deserialize)]
struct RawChatMessage {
    role: Role,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    tool_call_id: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<ToolCall>>,
    #[serde(default)]
    function_call: Option<RawFunctionCall>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleClient {
    base_url: String,
    api_key: Option<String>,
    http: reqwest::Client,
}

impl OpenAiCompatibleClient {
    pub fn new(base_url: String, api_key: Option<String>) -> anyhow::Result<Self> {
        let base_url = base_url.trim_end_matches('/').to_string();
        let timeout = openai_compat_timeout();
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .context("build http client")?;

        Ok(Self {
            base_url,
            api_key,
            http,
        })
    }

    pub async fn chat_completions(&self, req: ChatCompletionRequest) -> anyhow::Result<ChatMessage> {
        let url = format!("{}/chat/completions", self.base_url);

        let mut http_req = self.http.post(url).json(&req);
        if let Some(key) = &self.api_key {
            if !key.is_empty() {
                http_req = http_req.bearer_auth(key);
            }
        }

        let resp = http_req
            .send()
            .await
            .context("send chat completion request")?
            .error_for_status()
            .context("chat completion HTTP error")?;

        let body: RawChatCompletionResponse = resp
            .json()
            .await
            .context("decode chat completion response")?;

        let choice = body.choices.into_iter().next().context("no choices")?;
        let raw = choice.message;

        let mut tool_calls = raw.tool_calls;
        if tool_calls.as_ref().map(|c| c.is_empty()).unwrap_or(true) {
            if let Some(fc) = raw.function_call {
                tool_calls = Some(vec![ToolCall {
                    id: "call_1".to_string(),
                    kind: "function".to_string(),
                    function: ToolFunction {
                        name: fc.name,
                        arguments: fc.arguments,
                    },
                }]);
            }
        }

        Ok(ChatMessage {
            role: raw.role,
            content: raw.content,
            name: raw.name,
            tool_call_id: raw.tool_call_id,
            tool_calls,
        })
    }
}

fn openai_compat_timeout() -> Duration {
    const DEFAULT_SECS: u64 = 600;
    match std::env::var("REXOS_OPENAI_COMPAT_TIMEOUT_SECS") {
        Ok(raw) => match raw.trim().parse::<u64>() {
            Ok(secs) if secs > 0 => Duration::from_secs(secs),
            _ => Duration::from_secs(DEFAULT_SECS),
        },
        Err(_) => Duration::from_secs(DEFAULT_SECS),
    }
}
