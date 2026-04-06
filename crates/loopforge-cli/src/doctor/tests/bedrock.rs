use rexos::config::RexosConfig;

use super::common::{run_doctor_with_timeout, status_map, test_paths, write_config};

#[tokio::test]
async fn doctor_reports_bedrock_feature_status_when_routed() {
    let (_tmp, paths) = test_paths();

    let mut cfg = RexosConfig::default();
    cfg.router.coding.provider = "bedrock".to_string();
    cfg.router.coding.model = "default".to_string();
    if let Some(provider) = cfg.providers.get_mut("bedrock") {
        provider.default_model = "anthropic.claude-3-5-sonnet-20241022-v2:0".to_string();
    }
    write_config(&paths, &cfg);

    let report = run_doctor_with_timeout(paths, 200).await;
    let statuses = status_map(&report);

    assert_eq!(
        statuses.get("bedrock.router.coding.model"),
        Some(&crate::doctor::CheckStatus::Ok)
    );
    assert_eq!(
        statuses.get("bedrock.providers.bedrock.region"),
        Some(&crate::doctor::CheckStatus::Ok)
    );
    assert_eq!(
        statuses.get("bedrock.feature"),
        Some(if cfg!(feature = "bedrock") {
            &crate::doctor::CheckStatus::Ok
        } else {
            &crate::doctor::CheckStatus::Error
        })
    );

    if !cfg!(feature = "bedrock") {
        assert!(
            report
                .next_actions
                .iter()
                .any(|item| item.contains("features bedrock")),
            "expected bedrock rebuild guidance, got: {:?}",
            report.next_actions
        );
    }
}

#[tokio::test]
async fn doctor_flags_missing_bedrock_model_and_region() {
    let (_tmp, paths) = test_paths();

    let mut cfg = RexosConfig::default();
    cfg.router.coding.provider = "bedrock".to_string();
    cfg.router.coding.model = "default".to_string();
    if let Some(provider) = cfg.providers.get_mut("bedrock") {
        provider.default_model = "".to_string();
        if let Some(aws) = provider.aws_bedrock.as_mut() {
            aws.region = "".to_string();
        }
    }
    write_config(&paths, &cfg);

    let report = run_doctor_with_timeout(paths, 200).await;
    let statuses = status_map(&report);

    assert_eq!(
        statuses.get("bedrock.router.coding.model"),
        Some(&crate::doctor::CheckStatus::Error)
    );
    assert_eq!(
        statuses.get("bedrock.providers.bedrock.region"),
        Some(&crate::doctor::CheckStatus::Error)
    );
}
