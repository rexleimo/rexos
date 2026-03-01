use std::collections::BTreeMap;
use std::sync::Arc;

use crate::config::{ProviderKind, RexosConfig};
use crate::llm::anthropic::AnthropicDriver;
use crate::llm::driver::{LlmDriver, OpenAiCompatDriver, UnimplementedDriver};

#[derive(Clone)]
pub struct LlmRegistry {
    drivers: BTreeMap<String, Arc<dyn LlmDriver>>,
}

impl std::fmt::Debug for LlmRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let keys: Vec<&str> = self.drivers.keys().map(|k| k.as_str()).collect();
        f.debug_struct("LlmRegistry")
            .field("providers", &keys)
            .finish()
    }
}

impl LlmRegistry {
    pub fn from_config(cfg: &RexosConfig) -> anyhow::Result<Self> {
        let mut drivers: BTreeMap<String, Arc<dyn LlmDriver>> = BTreeMap::new();

        for (name, p) in &cfg.providers {
            let driver: Arc<dyn LlmDriver> = match p.kind {
                ProviderKind::OpenAiCompatible => Arc::new(OpenAiCompatDriver::new(
                    p.base_url.clone(),
                    cfg.provider_api_key(name),
                )?),
                ProviderKind::Anthropic => Arc::new(AnthropicDriver::new(
                    p.base_url.clone(),
                    cfg.provider_api_key(name),
                )?),
                ProviderKind::Gemini => Arc::new(UnimplementedDriver::new("gemini")),
            };

            drivers.insert(name.clone(), driver);
        }

        Ok(Self { drivers })
    }

    pub fn driver(&self, name: &str) -> Option<Arc<dyn LlmDriver>> {
        self.drivers.get(name).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LlmConfig, ProviderConfig, ProviderKind};

    #[test]
    fn registry_builds_and_resolves_drivers() {
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
            "anthropic".to_string(),
            ProviderConfig {
                kind: ProviderKind::Anthropic,
                base_url: "http://127.0.0.1:1".to_string(),
                api_key_env: "ANTHROPIC_API_KEY".to_string(),
                default_model: "claude-test".to_string(),
            },
        );

        let cfg = RexosConfig {
            llm: LlmConfig::default(),
            providers,
            router: crate::config::RouterConfig::default(),
        };

        let registry = LlmRegistry::from_config(&cfg).unwrap();
        assert!(registry.driver("ollama").is_some());
        assert!(registry.driver("anthropic").is_some());
    }
}
