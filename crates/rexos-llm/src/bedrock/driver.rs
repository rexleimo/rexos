use std::sync::Arc;

use anyhow::Context;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_bedrockruntime::Client;
use rexos_kernel::config::AwsBedrockConfig;

use crate::driver::{ChatFuture, LlmDriver};
use crate::openai_compat::ChatCompletionRequest;

use super::request::{build_tool_config, convert_messages};
use super::response::map_response;

const DEFAULT_MAX_TOKENS: i32 = 1024;

#[derive(Debug, Clone)]
pub struct BedrockDriver {
    region: String,
    cross_region_prefix: String,
    profile: Option<String>,
    client: Arc<tokio::sync::Mutex<Option<Client>>>,
}

impl BedrockDriver {
    pub fn new(cfg: Option<&AwsBedrockConfig>) -> anyhow::Result<Self> {
        let cfg = cfg.cloned().unwrap_or_default();
        let region = cfg.region.trim().to_string();
        if region.is_empty() {
            anyhow::bail!("bedrock region is empty");
        }

        let profile = cfg.profile.trim().to_string().trim().to_string();
        let profile = if profile.is_empty() {
            None
        } else {
            Some(profile)
        };

        let cross_region_prefix = normalize_cross_region_prefix(&cfg.cross_region);

        Ok(Self {
            region,
            cross_region_prefix,
            profile,
            client: Arc::new(tokio::sync::Mutex::new(None)),
        })
    }

    async fn get_client(&self) -> anyhow::Result<Client> {
        let mut guard = self.client.lock().await;
        if let Some(client) = guard.clone() {
            return Ok(client);
        }

        let mut builder = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(self.region.clone()));
        if let Some(profile) = &self.profile {
            builder = builder.profile_name(profile);
        }

        let sdk_config = builder.load().await;
        let client = Client::new(&sdk_config);
        *guard = Some(client.clone());
        Ok(client)
    }

    fn model_id(&self, requested: &str) -> String {
        let requested = requested.trim();
        if requested.is_empty() {
            return String::new();
        }
        if self.cross_region_prefix.is_empty() || requested.starts_with(&self.cross_region_prefix) {
            requested.to_string()
        } else {
            format!("{}{}", self.cross_region_prefix, requested)
        }
    }
}

impl LlmDriver for BedrockDriver {
    fn chat(&self, req: ChatCompletionRequest) -> ChatFuture<'_> {
        Box::pin(async move {
            let model_id = self.model_id(&req.model);
            if model_id.is_empty() {
                anyhow::bail!("bedrock request model is empty");
            }

            let (system_blocks, messages) = convert_messages(&req.messages)?;
            if messages.is_empty() {
                anyhow::bail!("bedrock requires at least one user or assistant message");
            }

            let tool_config = build_tool_config(&req.tools)?;
            let client = self.get_client().await?;

            let mut builder = client
                .converse()
                .model_id(model_id)
                .set_system(if system_blocks.is_empty() {
                    None
                } else {
                    Some(system_blocks)
                })
                .set_messages(Some(messages));

            if let Some(tool_config) = tool_config {
                builder = builder.tool_config(tool_config);
            }

            let mut inf = aws_sdk_bedrockruntime::types::InferenceConfiguration::builder()
                .max_tokens(DEFAULT_MAX_TOKENS);
            if let Some(temp) = req.temperature {
                inf = inf.temperature(temp);
            }
            builder = builder.inference_config(inf.build());

            let response = builder.send().await.context("send bedrock request")?;

            map_response(response)
        })
    }
}

fn normalize_cross_region_prefix(value: &str) -> String {
    let value = value.trim().trim_end_matches('.');
    if value.is_empty() {
        String::new()
    } else {
        format!("{value}.")
    }
}
