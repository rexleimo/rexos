mod defaults;
mod storage;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::paths::RexosPaths;
use crate::secrets::SecretResolver;
use crate::security::SecurityConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RexosConfig {
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub providers: BTreeMap<String, ProviderConfig>,
    #[serde(default)]
    pub router: RouterConfig,
    #[serde(default)]
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LlmConfig {
    /// Base URL for OpenAI-compatible API (example: https://api.openai.com/v1)
    pub base_url: String,
    /// Environment variable name holding the API key.
    pub api_key_env: String,
    /// Default model name.
    pub model: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProviderKind {
    #[serde(rename = "openai_compatible")]
    OpenAiCompatible,
    #[serde(rename = "dashscope_native")]
    DashscopeNative,
    #[serde(rename = "zhipu_native")]
    ZhipuNative,
    #[serde(rename = "minimax_native")]
    MiniMaxNative,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "gemini")]
    Gemini,
    #[serde(rename = "bedrock")]
    Bedrock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AwsBedrockConfig {
    pub region: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub cross_region: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderConfig {
    pub kind: ProviderKind,
    pub base_url: String,
    pub api_key_env: String,
    pub default_model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aws_bedrock: Option<AwsBedrockConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RouterConfig {
    pub planning: RouteConfig,
    pub coding: RouteConfig,
    pub summary: RouteConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RouteConfig {
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SkillsConfig {
    pub allowlist: Vec<String>,
    pub require_approval: bool,
    pub auto_approve_readonly: bool,
    pub experimental: bool,
}

impl Default for RexosConfig {
    fn default() -> Self {
        let providers = defaults::default_providers();
        Self {
            llm: LlmConfig::default(),
            providers: providers.clone(),
            router: RouterConfig::default_from_provider("ollama", &providers),
            security: SecurityConfig::default(),
        }
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        defaults::default_llm_config()
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        defaults::default_provider_config()
    }
}

impl Default for AwsBedrockConfig {
    fn default() -> Self {
        Self {
            region: "us-east-1".to_string(),
            cross_region: String::new(),
            profile: String::new(),
        }
    }
}

impl RouterConfig {
    fn default_from_provider(
        default_provider: &str,
        providers: &BTreeMap<String, ProviderConfig>,
    ) -> Self {
        defaults::default_router_config(default_provider, providers)
    }
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self::default_from_provider("ollama", &BTreeMap::new())
    }
}

impl Default for RouteConfig {
    fn default() -> Self {
        defaults::default_route_config()
    }
}

impl Default for SkillsConfig {
    fn default() -> Self {
        defaults::default_skills_config()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
struct SkillsConfigWrapper {
    skills: SkillsConfig,
}

impl RexosConfig {
    pub fn ensure_default(paths: &RexosPaths) -> anyhow::Result<()> {
        storage::ensure_default_config(paths)
    }

    pub fn load(paths: &RexosPaths) -> anyhow::Result<Self> {
        storage::load_config(paths)
    }

    pub fn api_key(&self) -> Option<String> {
        SecretResolver::new().resolve_llm_api_key(self)
    }

    pub fn provider_api_key(&self, provider: &str) -> Option<String> {
        SecretResolver::new().resolve_provider_api_key(self, provider)
    }

    pub fn load_skills_config(paths: &RexosPaths) -> anyhow::Result<SkillsConfig> {
        storage::load_skills_config(paths)
    }
}
