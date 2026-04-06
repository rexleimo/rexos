use std::time::Duration;

use anyhow::Context;

use crate::driver::{ChatFuture, LlmDriver};
use crate::openai_compat::ChatCompletionRequest;

use super::request::build_request;
use super::response::map_response;
use super::types::GeminiResponse;

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
    fn chat(&self, req: ChatCompletionRequest) -> ChatFuture<'_> {
        Box::pin(async move {
            let gemini_req = build_request(&req)?;

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
