use std::time::Duration;

use anyhow::Context;

use crate::driver::{ChatFuture, LlmDriver};
use crate::openai_compat::ChatCompletionRequest;

use super::auth::to_bearer_token;
use super::mapping::map_message;
use super::types::{RawChatCompletionResponse, ZhipuRequest};

#[derive(Debug, Clone)]
pub struct ZhipuDriver {
    base_url: String,
    api_key: Option<String>,
    http: reqwest::Client,
}

impl ZhipuDriver {
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

impl LlmDriver for ZhipuDriver {
    fn chat(&self, req: ChatCompletionRequest) -> ChatFuture<'_> {
        Box::pin(async move {
            let token = self.api_key.as_deref().and_then(to_bearer_token);

            let zhipu_req = ZhipuRequest {
                model: req.model,
                messages: req.messages,
                tools: req.tools,
                temperature: req.temperature,
                stream: false,
            };

            let url = format!("{}/chat/completions", self.base_url);
            let mut http_req = self.http.post(url).json(&zhipu_req);
            if let Some(token) = token {
                if !token.trim().is_empty() {
                    http_req = http_req.bearer_auth(token);
                }
            }

            let resp = http_req
                .send()
                .await
                .context("send zhipu request")?
                .error_for_status()
                .context("zhipu HTTP error")?;

            let body: RawChatCompletionResponse =
                resp.json().await.context("decode zhipu response")?;
            let choice = body.choices.into_iter().next().context("no choices")?;
            map_message(choice.message)
        })
    }
}
