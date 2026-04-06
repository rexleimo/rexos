use std::time::Duration;

use anyhow::Context;

use crate::driver::{ChatFuture, LlmDriver};
use crate::openai_compat::ChatCompletionRequest;

use super::mapping::{clean_message, map_messages};
use super::types::{DashscopeInput, DashscopeParameters, DashscopeRequest, DashscopeResponse};

#[derive(Debug, Clone)]
pub struct DashscopeDriver {
    base_url: String,
    api_key: Option<String>,
    http: reqwest::Client,
}

impl DashscopeDriver {
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

impl LlmDriver for DashscopeDriver {
    fn chat(&self, req: ChatCompletionRequest) -> ChatFuture<'_> {
        Box::pin(async move {
            let dash_req = DashscopeRequest {
                model: req.model,
                input: DashscopeInput {
                    messages: map_messages(&req.messages),
                },
                parameters: DashscopeParameters {
                    result_format: "message".to_string(),
                    temperature: req.temperature,
                    tools: req.tools,
                },
            };

            let url = format!("{}/services/aigc/text-generation/generation", self.base_url);
            let mut http_req = self.http.post(url).json(&dash_req);
            if let Some(key) = &self.api_key {
                if !key.trim().is_empty() {
                    http_req = http_req.bearer_auth(key);
                }
            }

            let resp = http_req
                .send()
                .await
                .context("send dashscope request")?
                .error_for_status()
                .context("dashscope HTTP error")?;

            let body: DashscopeResponse = resp.json().await.context("decode dashscope response")?;
            let choice = body
                .output
                .choices
                .into_iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("no choices"))?;

            Ok(clean_message(choice.message))
        })
    }
}
