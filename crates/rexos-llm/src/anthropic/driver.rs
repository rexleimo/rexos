use std::time::Duration;

use anyhow::Context;

use crate::driver::{ChatFuture, LlmDriver};
use crate::openai_compat::ChatCompletionRequest;

use super::request::build_request;
use super::response::map_response;
use super::types::AnthropicResponse;

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
    fn chat(&self, req: ChatCompletionRequest) -> ChatFuture<'_> {
        Box::pin(async move {
            let anthropic_req = build_request(req)?;

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
