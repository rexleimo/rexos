use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::llm::driver::{ChatFuture, LlmDriver};
use crate::llm::openai_compat::{ChatCompletionRequest, ChatMessage, Role, ToolCall, ToolDefinition, ToolFunction};

#[derive(Debug, Clone)]
pub struct GeminiDriver {
    base_url: String,
    api_key: Option<String>,
    http: reqwest::Client,
}

impl GeminiDriver {
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

impl LlmDriver for GeminiDriver {
    fn chat<'a>(&'a self, req: ChatCompletionRequest) -> ChatFuture<'a> {
        Box::pin(async move {
            let (system_instruction, contents) = map_messages(&req.messages)?;
            let tools = map_tools(&req.tools);

            let gemini_req = GeminiRequest {
                contents,
                system_instruction,
                tools,
            };

            let url = format!("{}/models/{}:generateContent", self.base_url, req.model);
            let mut http_req = self.http.post(url).json(&gemini_req);
            if let Some(key) = &self.api_key {
                if !key.trim().is_empty() {
                    http_req = http_req.query(&[("key", key)]);
                }
            }

            let resp = http_req
                .send()
                .await
                .context("send gemini request")?
                .error_for_status()
                .context("gemini HTTP error")?;

            let body: GeminiResponse = resp.json().await.context("decode gemini response")?;
            map_response(body)
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiSystemInstruction>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    tools: Vec<GeminiTool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiSystemInstruction {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiTool {
    function_declarations: Vec<GeminiFunctionDeclaration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiFunctionDeclaration {
    name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum GeminiPart {
    Text { text: String },
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: GeminiFunctionCall,
    },
    FunctionResponse {
        #[serde(rename = "functionResponse")]
        function_response: GeminiFunctionResponse,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiFunctionCall {
    name: String,
    #[serde(default)]
    args: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiFunctionResponse {
    name: String,
    response: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiResponse {
    #[serde(default)]
    candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiCandidate {
    content: GeminiContent,
}

fn map_tools(tools: &[ToolDefinition]) -> Vec<GeminiTool> {
    let decls: Vec<GeminiFunctionDeclaration> = tools
        .iter()
        .filter_map(|t| {
            if t.kind != "function" {
                return None;
            }
            Some(GeminiFunctionDeclaration {
                name: t.function.name.clone(),
                description: t.function.description.clone(),
                parameters: t.function.parameters.clone(),
            })
        })
        .collect();

    if decls.is_empty() {
        Vec::new()
    } else {
        vec![GeminiTool {
            function_declarations: decls,
        }]
    }
}

fn map_messages(messages: &[ChatMessage]) -> anyhow::Result<(Option<GeminiSystemInstruction>, Vec<GeminiContent>)> {
    let mut system_parts = Vec::new();
    let mut out = Vec::new();
    let mut tool_name_by_id: BTreeMap<String, String> = BTreeMap::new();

    for m in messages {
        if let Role::System = m.role {
            if let Some(s) = m.content.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
                system_parts.push(s.to_string());
            }
        }

        if let Role::Assistant = m.role {
            if let Some(calls) = &m.tool_calls {
                for c in calls {
                    tool_name_by_id.insert(c.id.clone(), c.function.name.clone());
                }
            }
        }
    }

    for m in messages {
        match m.role {
            Role::System => {}
            Role::User => {
                let text = m.content.as_ref().map(|s| s.trim()).unwrap_or("");
                if !text.is_empty() {
                    out.push(GeminiContent {
                        role: "user".to_string(),
                        parts: vec![GeminiPart::Text { text: text.to_string() }],
                    });
                }
            }
            Role::Assistant => {
                let mut parts = Vec::new();
                let text = m.content.as_ref().map(|s| s.trim()).unwrap_or("");
                if !text.is_empty() {
                    parts.push(GeminiPart::Text { text: text.to_string() });
                }
                if let Some(calls) = &m.tool_calls {
                    for c in calls {
                        let args = serde_json::from_str::<serde_json::Value>(&c.function.arguments)
                            .unwrap_or(serde_json::Value::Null);
                        parts.push(GeminiPart::FunctionCall {
                            function_call: GeminiFunctionCall {
                                name: c.function.name.clone(),
                                args,
                            },
                        });
                    }
                }
                if !parts.is_empty() {
                    out.push(GeminiContent {
                        role: "model".to_string(),
                        parts,
                    });
                }
            }
            Role::Tool => {
                let tool_use_id = m
                    .tool_call_id
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("tool message missing tool_call_id"))?;
                let name = tool_name_by_id
                    .get(tool_use_id)
                    .ok_or_else(|| anyhow::anyhow!("unknown tool_call_id: {tool_use_id}"))?
                    .clone();
                let output = m.content.clone().unwrap_or_default();
                out.push(GeminiContent {
                    role: "function".to_string(),
                    parts: vec![GeminiPart::FunctionResponse {
                        function_response: GeminiFunctionResponse {
                            name,
                            response: serde_json::json!({ "output": output }),
                        },
                    }],
                });
            }
        }
    }

    let system_instruction = if system_parts.is_empty() {
        None
    } else {
        Some(GeminiSystemInstruction {
            parts: vec![GeminiPart::Text {
                text: system_parts.join("\n\n"),
            }],
        })
    };

    Ok((system_instruction, out))
}

fn map_response(resp: GeminiResponse) -> anyhow::Result<ChatMessage> {
    let first = resp
        .candidates
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("no candidates"))?;

    let mut texts = Vec::new();
    let mut tool_calls = Vec::new();

    for (idx, p) in first.content.parts.into_iter().enumerate() {
        match p {
            GeminiPart::Text { text } => {
                if !text.trim().is_empty() {
                    texts.push(text);
                }
            }
            GeminiPart::FunctionCall { function_call } => {
                tool_calls.push(ToolCall {
                    id: format!("call_{}", idx + 1),
                    kind: "function".to_string(),
                    function: ToolFunction {
                        name: function_call.name,
                        arguments: serde_json::to_string(&function_call.args)?,
                    },
                });
            }
            GeminiPart::FunctionResponse { .. } => {}
        }
    }

    Ok(ChatMessage {
        role: Role::Assistant,
        content: if texts.is_empty() { None } else { Some(texts.join("\n")) },
        name: None,
        tool_call_id: None,
        tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
    })
}
