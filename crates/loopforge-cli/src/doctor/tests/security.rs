use rexos::config::{ProviderKind, RexosConfig};

use super::common::{run_doctor_with_timeout, status_map, test_paths, write_config};

#[tokio::test]
async fn doctor_reports_security_posture_checks() {
    let (_tmp, paths) = test_paths();

    let mut cfg = RexosConfig {
        llm: rexos::config::LlmConfig::default(),
        providers: [(
            "ollama".to_string(),
            rexos::config::ProviderConfig {
                kind: ProviderKind::OpenAiCompatible,
                base_url: "http://127.0.0.1:11434/v1".to_string(),
                api_key_env: "".to_string(),
                default_model: "x".to_string(),
                aws_bedrock: None,
            },
        )]
        .into_iter()
        .collect(),
        router: rexos::config::RouterConfig::default(),
        security: Default::default(),
    };
    cfg.security.leaks.mode = rexos::security::LeakMode::Redact;
    cfg.security.egress.rules.push(rexos::security::EgressRule {
        tool: "web_fetch".to_string(),
        host: "docs.rs".to_string(),
        path_prefix: "/".to_string(),
        methods: vec!["GET".to_string()],
    });
    write_config(&paths, &cfg);

    let report = run_doctor_with_timeout(paths, 200).await;
    let statuses = status_map(&report);
    assert_eq!(
        statuses.get("security.secrets.mode"),
        Some(&crate::doctor::CheckStatus::Ok)
    );
    assert_eq!(
        statuses.get("security.leaks.mode"),
        Some(&crate::doctor::CheckStatus::Ok)
    );
    assert_eq!(
        statuses.get("security.egress.rules"),
        Some(&crate::doctor::CheckStatus::Ok)
    );
}

#[tokio::test]
async fn doctor_suggests_leak_guard_and_egress_hardening_when_defaults_are_open() {
    let (_tmp, paths) = test_paths();

    let cfg = RexosConfig {
        llm: rexos::config::LlmConfig::default(),
        providers: [(
            "ollama".to_string(),
            rexos::config::ProviderConfig {
                kind: ProviderKind::OpenAiCompatible,
                base_url: "http://127.0.0.1:11434/v1".to_string(),
                api_key_env: "".to_string(),
                default_model: "x".to_string(),
                aws_bedrock: None,
            },
        )]
        .into_iter()
        .collect(),
        router: rexos::config::RouterConfig::default(),
        security: Default::default(),
    };
    write_config(&paths, &cfg);

    let report = run_doctor_with_timeout(paths, 200).await;
    let statuses = status_map(&report);
    assert_eq!(
        statuses.get("security.leaks.mode"),
        Some(&crate::doctor::CheckStatus::Warn)
    );
    assert_eq!(
        statuses.get("security.egress.rules"),
        Some(&crate::doctor::CheckStatus::Warn)
    );
    assert!(
        report
            .next_actions
            .iter()
            .any(|item| item.contains("security.leaks")),
        "expected leak-guard guidance, got: {:?}",
        report.next_actions
    );
    assert!(
        report
            .next_actions
            .iter()
            .any(|item| item.contains("security.egress")),
        "expected egress guidance, got: {:?}",
        report.next_actions
    );
}
