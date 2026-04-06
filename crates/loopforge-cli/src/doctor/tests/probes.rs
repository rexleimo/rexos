use axum::routing::get;
use axum::{Json, Router};
use rexos::config::{ProviderKind, RexosConfig};

use super::common::{run_doctor_with_timeout, status_map, test_paths, write_config};

#[tokio::test]
async fn doctor_probes_local_ollama_models_and_cdp_version() {
    async fn models() -> Json<serde_json::Value> {
        Json(serde_json::json!({ "data": [] }))
    }
    async fn cdp_version() -> Json<serde_json::Value> {
        Json(serde_json::json!({ "Browser": "Chrome/1.0" }))
    }

    let app = Router::new()
        .route("/v1/models", get(models))
        .route("/json/version", get(cdp_version));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let (_tmp, paths) = test_paths();

    let cfg = RexosConfig {
        llm: rexos::config::LlmConfig::default(),
        providers: [(
            "ollama".to_string(),
            rexos::config::ProviderConfig {
                kind: ProviderKind::OpenAiCompatible,
                base_url: format!("http://{addr}/v1"),
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
    std::env::set_var("LOOPFORGE_BROWSER_CDP_HTTP", format!("http://{addr}"));

    let report = run_doctor_with_timeout(paths, 500).await;
    let statuses = status_map(&report);
    assert_eq!(
        statuses.get("ollama.http"),
        Some(&crate::doctor::CheckStatus::Ok)
    );
    assert_eq!(
        statuses.get("browser.cdp_http"),
        Some(&crate::doctor::CheckStatus::Ok)
    );

    std::env::remove_var("LOOPFORGE_BROWSER_CDP_HTTP");
    server.abort();
}
