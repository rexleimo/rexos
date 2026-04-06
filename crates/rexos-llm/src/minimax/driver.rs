use std::time::Duration;

use anyhow::Context;

use crate::driver::{ChatFuture, LlmDriver};
use crate::openai_compat::ChatCompletionRequest;

use super::mapping::map_message;
use super::types::{MiniMaxRequest, RawChatCompletionResponse};

#[derive(Debug, Clone)]
pub struct MiniMaxDriver {
    base_url: String,
    api_key: Option<String>,
    http: reqwest::Client,
}

impl MiniMaxDriver {
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

impl LlmDriver for MiniMaxDriver {
    fn chat(&self, req: ChatCompletionRequest) -> ChatFuture<'_> {
        Box::pin(async move {
            let tool_choice = if req.tools.is_empty() {
                "none".to_string()
            } else {
                "auto".to_string()
            };

            let mm_req = MiniMaxRequest {
                model: req.model,
                messages: req.messages,
                stream: false,
                temperature: req.temperature,
                tools: req.tools,
                tool_choice,
            };

            let url = format!("{}/text/chatcompletion_v2", self.base_url);
            let mut http_req = self.http.post(url).json(&mm_req);
            if let Some(key) = &self.api_key {
                if !key.trim().is_empty() {
                    http_req = http_req.bearer_auth(key);
                }
            }

            let resp = http_req
                .send()
                .await
                .context("send minimax request")?
                .error_for_status()
                .context("minimax HTTP error")?;

            let body: RawChatCompletionResponse =
                resp.json().await.context("decode minimax response")?;
            let choice = body.choices.into_iter().next().context("no choices")?;
            map_message(choice.message)
        })
    }
}
