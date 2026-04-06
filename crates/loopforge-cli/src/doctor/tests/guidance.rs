use rexos::config::{ProviderKind, RexosConfig};

use super::common::{run_doctor_with_timeout, test_paths, write_config};

#[tokio::test]
async fn doctor_suggests_running_init_when_core_files_are_missing() {
    let (_tmp, paths) = test_paths();

    let report = run_doctor_with_timeout(paths, 200).await;

    let value = serde_json::to_value(&report).unwrap();
    let next_actions = value
        .get("next_actions")
        .and_then(|item| item.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        next_actions
            .iter()
            .any(|item| item.as_str().unwrap_or("").contains("loopforge init")),
        "expected init guidance in next_actions, got: {next_actions:?}"
    );
    assert!(
        report.to_text().contains("Suggested next steps"),
        "expected text output to include suggested next steps, got: {}",
        report.to_text()
    );
}

#[tokio::test]
async fn doctor_suggests_missing_provider_env_vars() {
    let (_tmp, paths) = test_paths();

    let mut cfg = RexosConfig::default();
    cfg.providers.insert(
        "anthropic".to_string(),
        rexos::config::ProviderConfig {
            kind: ProviderKind::Anthropic,
            base_url: "https://api.anthropic.com".to_string(),
            api_key_env: "ANTHROPIC_API_KEY".to_string(),
            default_model: "claude-3-5-sonnet-latest".to_string(),
            aws_bedrock: None,
        },
    );
    write_config(&paths, &cfg);
    std::env::remove_var("ANTHROPIC_API_KEY");

    let report = run_doctor_with_timeout(paths, 200).await;

    let value = serde_json::to_value(&report).unwrap();
    let next_actions = value
        .get("next_actions")
        .and_then(|item| item.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        next_actions
            .iter()
            .any(|item| item.as_str().unwrap_or("").contains("ANTHROPIC_API_KEY")),
        "expected provider env guidance in next_actions, got: {next_actions:?}"
    );
}
