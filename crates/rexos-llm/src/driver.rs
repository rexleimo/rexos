use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::bail;

use crate::openai_compat::{ChatCompletionRequest, ChatMessage, OpenAiCompatibleClient};

pub type ChatFuture<'a> = Pin<Box<dyn Future<Output = anyhow::Result<ChatMessage>> + Send + 'a>>;

pub trait LlmDriver: Send + Sync {
    fn chat(&self, req: ChatCompletionRequest) -> ChatFuture<'_>;
}

#[derive(Clone)]
pub struct OpenAiCompatDriver {
    client: OpenAiCompatibleClient,
}

impl OpenAiCompatDriver {
    pub fn new(base_url: String, api_key: Option<String>) -> anyhow::Result<Self> {
        Ok(Self {
            client: OpenAiCompatibleClient::new(base_url, api_key)?,
        })
    }
}

impl LlmDriver for OpenAiCompatDriver {
    fn chat(&self, req: ChatCompletionRequest) -> ChatFuture<'_> {
        Box::pin(async move { self.client.chat_completions(req).await })
    }
}

#[derive(Clone)]
pub struct UnimplementedDriver {
    provider: Arc<str>,
}

impl UnimplementedDriver {
    pub fn new(provider: impl Into<Arc<str>>) -> Self {
        Self {
            provider: provider.into(),
        }
    }
}

impl LlmDriver for UnimplementedDriver {
    fn chat(&self, _req: ChatCompletionRequest) -> ChatFuture<'_> {
        Box::pin(async move { bail!("provider not implemented: {}", self.provider) })
    }
}
