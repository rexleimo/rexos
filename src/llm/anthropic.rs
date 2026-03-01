use std::time::Duration;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::llm::driver::{ChatFuture, LlmDriver};
use crate::llm::openai_compat::{ChatCompletionRequest, ChatMessage, Role, ToolCall, ToolDefinition};
use crate::llm::openai_compat::ToolFunction;

#[derive(Debug, Clone)]
pub struct AnthropicDriver {
    base_url: String,
    api_key: Option<String>,
    http: reqwest::Client,
}

impl AnthropicDriver {
    pub fn new(base_url: String, api_key: Option<String>) -> anyhow::Result<Self> {
        let base_url = base_url.trim_end_matches('/').to_string();
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .context("build http client")?;
        Ok(Self {
            base_url,
            api_key,
            http,
        })
    }
}

impl LlmDriver for AnthropicDriver {
    fn chat<'a>(&'a self, req: ChatCompletionRequest) -> ChatFuture<'a> {
        Box::pin(async move {
            let (system, messages) = map_messages(&req.messages)?;
            let tools = map_tools(&req.tools);

            let anthropic_req = AnthropicRequest {
                model: req.model,
                max_tokens: 1024,
                system,
                messages,
                tools,
            };

            let url = format!("{}/v1/messages", self.base_url);
            let mut http_req = self.http.post(url).json(&anthropic_req);
            http_req = http_req.header("anthropic-version", "2023-06-01");
            if let Some(key) = &self.api_key {
                if !key.trim().is_empty() {
                    http_req = http_req.header("x-api-key", key);
                }
            }

            let resp = http_req
                .send()
                .await
                .context("send anthropic request")?
                .error_for_status()
                .context("anthropic HTTP error")?;

            let body: AnthropicResponse = resp.json().await.context("decode anthropic response")?;
            map_response(body)
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    system: String,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    tools: Vec<AnthropicTool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum AnthropicContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        #[serde(default)]
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicTool {
    name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    description: String,
    #[serde(rename = "input_schema")]
    input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
struct AnthropicResponse {
    #[serde(default)]
    content: Vec<AnthropicContentBlock>,
}

fn map_tools(tools: &[ToolDefinition]) -> Vec<AnthropicTool> {
    tools
        .iter()
        .filter_map(|t| {
            if t.kind != "function" {
                return None;
            }
            Some(AnthropicTool {
                name: t.function.name.clone(),
                description: t.function.description.clone(),
                input_schema: t.function.parameters.clone(),
            })
        })
        .collect()
}

fn map_messages(messages: &[ChatMessage]) -> anyhow::Result<(String, Vec<AnthropicMessage>)> {
    let mut system_parts: Vec<String> = Vec::new();
    let mut out: Vec<AnthropicMessage> = Vec::new();

    for m in messages {
        match m.role {
            Role::System => {
                if let Some(s) = m.content.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    system_parts.push(s.to_string());
                }
            }
            Role::User => {
                let mut blocks = Vec::new();
                if let Some(s) = m.content.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    blocks.push(AnthropicContentBlock::Text { text: s.to_string() });
                }
                if !blocks.is_empty() {
                    out.push(AnthropicMessage {
                        role: "user".to_string(),
                        content: blocks,
                    });
                }
            }
            Role::Assistant => {
                let mut blocks = Vec::new();
                if let Some(s) = m.content.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    blocks.push(AnthropicContentBlock::Text { text: s.to_string() });
                }

                if let Some(calls) = &m.tool_calls {
                    for c in calls {
                        let input = serde_json::from_str::<serde_json::Value>(&c.function.arguments)
                            .unwrap_or(serde_json::Value::Null);
                        blocks.push(AnthropicContentBlock::ToolUse {
                            id: c.id.clone(),
                            name: c.function.name.clone(),
                            input,
                        });
                    }
                }

                if !blocks.is_empty() {
                    out.push(AnthropicMessage {
                        role: "assistant".to_string(),
                        content: blocks,
                    });
                }
            }
            Role::Tool => {
                let tool_use_id = m
                    .tool_call_id
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("tool message missing tool_call_id"))?;
                let content = m.content.clone().unwrap_or_default();

                out.push(AnthropicMessage {
                    role: "user".to_string(),
                    content: vec![AnthropicContentBlock::ToolResult {
                        tool_use_id: tool_use_id.to_string(),
                        content,
                        is_error: None,
                    }],
                });
            }
        }
    }

    Ok((system_parts.join("\n\n"), out))
}

fn map_response(resp: AnthropicResponse) -> anyhow::Result<ChatMessage> {
    let mut texts = Vec::new();
    let mut tool_calls: Vec<ToolCall> = Vec::new();

    for b in resp.content {
        match b {
            AnthropicContentBlock::Text { text } => {
                if !text.trim().is_empty() {
                    texts.push(text);
                }
            }
            AnthropicContentBlock::ToolUse { id, name, input } => {
                tool_calls.push(ToolCall {
                    id,
                    kind: "function".to_string(),
                    function: ToolFunction {
                        name,
                        arguments: serde_json::to_string(&input)?,
                    },
                });
            }
            AnthropicContentBlock::ToolResult { .. } => {}
        }
    }

    let content = if texts.is_empty() {
        None
    } else {
        Some(texts.join("\n"))
    };

    Ok(ChatMessage {
        role: Role::Assistant,
        content,
        name: None,
        tool_call_id: None,
        tool_calls: if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls)
        },
    })
}
