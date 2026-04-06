use std::collections::BTreeMap;

use rexos_kernel::config::{LlmConfig, ProviderConfig, ProviderKind, RexosConfig, RouterConfig};

use super::LlmRegistry;

#[test]
fn secret_resolver_reads_provider_key_from_env() {
    use rexos_kernel::secrets::SecretResolver;

    let env_name = format!("LOOPFORGE_TEST_SECRET_RESOLVER_{}", std::process::id());
    std::env::set_var(&env_name, "test-secret");

    let mut providers = BTreeMap::new();
    providers.insert(
        "anthropic".to_string(),
        ProviderConfig {
            kind: ProviderKind::Anthropic,
            base_url: "http://127.0.0.1:1".to_string(),
            api_key_env: env_name.clone(),
            default_model: "claude-test".to_string(),
            aws_bedrock: None,
        },
    );

    let cfg = RexosConfig {
        llm: LlmConfig::default(),
        providers,
        router: RouterConfig::default(),
        security: Default::default(),
    };

    let resolver = SecretResolver;
    assert_eq!(
        resolver.resolve_provider_api_key(&cfg, "anthropic"),
        Some("test-secret".to_string())
    );

    std::env::remove_var(&env_name);
}

#[test]
fn secret_resolver_returns_none_for_blank_env_name() {
    use rexos_kernel::secrets::SecretResolver;

    let resolver = SecretResolver;
    assert_eq!(resolver.resolve_env(""), None);
}

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
            aws_bedrock: None,
        },
    );
    providers.insert(
        "qwen_native".to_string(),
        ProviderConfig {
            kind: ProviderKind::DashscopeNative,
            base_url: "http://127.0.0.1:1/api/v1".to_string(),
            api_key_env: "DASHSCOPE_API_KEY".to_string(),
            default_model: "qwen-plus".to_string(),
            aws_bedrock: None,
        },
    );
    providers.insert(
        "glm_native".to_string(),
        ProviderConfig {
            kind: ProviderKind::ZhipuNative,
            base_url: "http://127.0.0.1:1/api/paas/v4".to_string(),
            api_key_env: "ZHIPUAI_API_KEY".to_string(),
            default_model: "glm-4".to_string(),
            aws_bedrock: None,
        },
    );
    providers.insert(
        "minimax_native".to_string(),
        ProviderConfig {
            kind: ProviderKind::MiniMaxNative,
            base_url: "http://127.0.0.1:1/v1".to_string(),
            api_key_env: "MINIMAX_API_KEY".to_string(),
            default_model: "MiniMax-M2.5".to_string(),
            aws_bedrock: None,
        },
    );
    providers.insert(
        "anthropic".to_string(),
        ProviderConfig {
            kind: ProviderKind::Anthropic,
            base_url: "http://127.0.0.1:1".to_string(),
            api_key_env: "ANTHROPIC_API_KEY".to_string(),
            default_model: "claude-test".to_string(),
            aws_bedrock: None,
        },
    );

    let cfg = RexosConfig {
        llm: LlmConfig::default(),
        providers,
        router: RouterConfig::default(),
        security: Default::default(),
    };

    let registry = LlmRegistry::from_config(&cfg).unwrap();
    assert!(registry.driver("ollama").is_some());
    assert!(registry.driver("qwen_native").is_some());
    assert!(registry.driver("glm_native").is_some());
    assert!(registry.driver("minimax_native").is_some());
    assert!(registry.driver("anthropic").is_some());
}
