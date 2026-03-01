use std::collections::BTreeMap;
use std::fs;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::paths::RexosPaths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RexosConfig {
    pub llm: LlmConfig,
    #[serde(default)]
    pub providers: BTreeMap<String, ProviderConfig>,
    #[serde(default)]
    pub router: RouterConfig,
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
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "gemini")]
    Gemini,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderConfig {
    pub kind: ProviderKind,
    pub base_url: String,
    pub api_key_env: String,
    pub default_model: String,
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

impl Default for RexosConfig {
    fn default() -> Self {
        let mut providers = BTreeMap::new();
        providers.insert(
            "ollama".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompatible,
                base_url: "http://127.0.0.1:11434/v1".to_string(),
                api_key_env: "".to_string(),
                default_model: "llama3.2".to_string(),
            },
        );
        providers.insert(
            "deepseek".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompatible,
                base_url: "https://api.deepseek.com/v1".to_string(),
                api_key_env: "DEEPSEEK_API_KEY".to_string(),
                default_model: "deepseek-chat".to_string(),
            },
        );
        providers.insert(
            "kimi".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompatible,
                base_url: "https://api.moonshot.cn/v1".to_string(),
                api_key_env: "MOONSHOT_API_KEY".to_string(),
                default_model: "moonshot-v1-8k".to_string(),
            },
        );
        providers.insert(
            "qwen".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompatible,
                base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string(),
                api_key_env: "DASHSCOPE_API_KEY".to_string(),
                default_model: "qwen-plus".to_string(),
            },
        );
        providers.insert(
            "glm".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompatible,
                base_url: "https://open.bigmodel.cn/api/paas/v4".to_string(),
                api_key_env: "ZHIPUAI_API_KEY".to_string(),
                default_model: "glm-5".to_string(),
            },
        );
        providers.insert(
            "minimax".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompatible,
                base_url: "https://api.minimax.io/v1".to_string(),
                api_key_env: "MINIMAX_API_KEY".to_string(),
                default_model: "MiniMax-Text-01".to_string(),
            },
        );
        providers.insert(
            "anthropic".to_string(),
            ProviderConfig {
                kind: ProviderKind::Anthropic,
                base_url: "https://api.anthropic.com".to_string(),
                api_key_env: "ANTHROPIC_API_KEY".to_string(),
                default_model: "claude-3-5-sonnet-latest".to_string(),
            },
        );
        providers.insert(
            "gemini".to_string(),
            ProviderConfig {
                kind: ProviderKind::Gemini,
                base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
                api_key_env: "GEMINI_API_KEY".to_string(),
                default_model: "gemini-1.5-flash".to_string(),
            },
        );

        Self {
            llm: LlmConfig::default(),
            providers: providers.clone(),
            router: RouterConfig::default_from_provider("ollama", &providers),
        }
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:11434/v1".to_string(),
            api_key_env: "OPENAI_API_KEY".to_string(),
            model: "gpt-4.1-mini".to_string(),
        }
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            kind: ProviderKind::OpenAiCompatible,
            base_url: "".to_string(),
            api_key_env: "".to_string(),
            default_model: "".to_string(),
        }
    }
}

impl RouterConfig {
    fn default_from_provider(default_provider: &str, providers: &BTreeMap<String, ProviderConfig>) -> Self {
        let model = providers
            .get(default_provider)
            .map(|p| p.default_model.as_str())
            .unwrap_or("llama3.2");

        Self {
            planning: RouteConfig {
                provider: default_provider.to_string(),
                model: model.to_string(),
            },
            coding: RouteConfig {
                provider: default_provider.to_string(),
                model: model.to_string(),
            },
            summary: RouteConfig {
                provider: default_provider.to_string(),
                model: model.to_string(),
            },
        }
    }
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self::default_from_provider("ollama", &BTreeMap::new())
    }
}

impl Default for RouteConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            model: "llama3.2".to_string(),
        }
    }
}

impl RexosConfig {
    pub fn ensure_default(paths: &RexosPaths) -> anyhow::Result<()> {
        let config_path = paths.config_path();
        if config_path.exists() {
            return Ok(());
        }

        let default_config = RexosConfig::default();
        let toml_str = toml::to_string_pretty(&default_config).context("serialize config")?;

        fs::write(&config_path, toml_str)
            .with_context(|| format!("write config: {}", config_path.display()))?;
        Ok(())
    }

    pub fn load(paths: &RexosPaths) -> anyhow::Result<Self> {
        let config_path = paths.config_path();
        let raw = fs::read_to_string(&config_path)
            .with_context(|| format!("read config: {}", config_path.display()))?;
        toml::from_str(&raw).context("parse config TOML")
    }

    pub fn api_key(&self) -> Option<String> {
        if self.llm.api_key_env.trim().is_empty() {
            return None;
        }
        std::env::var(&self.llm.api_key_env).ok()
    }

    pub fn provider_api_key(&self, provider: &str) -> Option<String> {
        let env = self
            .providers
            .get(provider)
            .map(|p| p.api_key_env.as_str())
            .unwrap_or("");
        if env.trim().is_empty() {
            return None;
        }
        std::env::var(env).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_serializes() {
        let cfg = RexosConfig::default();
        let toml_str = toml::to_string_pretty(&cfg).unwrap();
        assert!(toml_str.contains("[providers.ollama]"));
        assert!(toml_str.contains("[providers.deepseek]"));
        assert!(toml_str.contains("[providers.kimi]"));
        assert!(toml_str.contains("[providers.qwen]"));
        assert!(toml_str.contains("[providers.glm]"));
        assert!(toml_str.contains("[providers.minimax]"));
        assert!(toml_str.contains("[providers.anthropic]"));
        assert!(toml_str.contains("[providers.gemini]"));
        assert!(toml_str.contains("kind = \"openai_compatible\""));
        assert!(toml_str.contains("base_url"));
        assert!(toml_str.contains("api_key_env"));
        assert!(toml_str.contains("default_model"));

        assert!(toml_str.contains("[router.planning]"));
        assert!(toml_str.contains("provider = \"ollama\""));
        assert!(toml_str.contains("[router.coding]"));
        assert!(toml_str.contains("[router.summary]"));
    }
}
