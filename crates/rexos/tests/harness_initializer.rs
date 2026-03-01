use std::collections::BTreeMap;
use std::process::Command;
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::json;

#[derive(Clone, Default)]
struct TestState {
    calls: Arc<Mutex<u32>>,
}

#[tokio::test]
async fn harness_bootstrap_with_prompt_populates_features_and_checkpoints() {
    async fn handler(
        State(state): State<TestState>,
        Json(_payload): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        let mut calls = state.calls.lock().unwrap();
        *calls += 1;

        if *calls == 1 {
            return Json(json!({
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": null,
                        "tool_calls": [{
                            "id": "call_1",
                            "type": "function",
                            "function": {
                                "name": "fs_write",
                                "arguments": serde_json::to_string(&json!({
                                    "path": "features.json",
                                    "content": serde_json::to_string_pretty(&json!({
                                        "version": 1,
                                        "features": [{
                                            "id": "feat_1",
                                            "description": "First feature",
                                            "steps": ["do thing"],
                                            "passes": false
                                        }]
                                    })).unwrap()
                                })).unwrap()
                            }
                        }]
                    },
                    "finish_reason": "tool_calls"
                }]
            }));
        }

        Json(json!({
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "ok" },
                "finish_reason": "stop"
            }]
        }))
    }

    let state = TestState::default();
    let app = Router::new()
        .route("/v1/chat/completions", post(handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("ws");

    let home = tmp.path().join("home");
    let paths = rexos::paths::RexosPaths {
        base_dir: home.join(".rexos"),
    };
    paths.ensure_dirs().unwrap();

    let memory = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();

    let mut providers = BTreeMap::new();
    providers.insert(
        "p".to_string(),
        rexos::config::ProviderConfig {
            kind: rexos::config::ProviderKind::OpenAiCompatible,
            base_url: format!("http://{addr}/v1"),
            api_key_env: "".to_string(),
            default_model: "x".to_string(),
        },
    );

    let cfg = rexos::config::RexosConfig {
        llm: rexos::config::LlmConfig::default(),
        providers,
        router: rexos::config::RouterConfig::default(),
    };
    let llms = rexos::llm::registry::LlmRegistry::from_config(&cfg).unwrap();
    let router = rexos::router::ModelRouter::new(rexos::config::RouterConfig {
        planning: rexos::config::RouteConfig {
            provider: "p".to_string(),
            model: "x".to_string(),
        },
        coding: rexos::config::RouteConfig {
            provider: "p".to_string(),
            model: "x".to_string(),
        },
        summary: rexos::config::RouteConfig {
            provider: "p".to_string(),
            model: "x".to_string(),
        },
    });

    let agent = rexos::agent::AgentRuntime::new(memory, llms, router);

    rexos::harness::bootstrap_with_prompt(&agent, &workspace, "s1", "make a plan")
        .await
        .unwrap();

    let raw = std::fs::read_to_string(workspace.join("features.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(v["features"].as_array().unwrap().len(), 1);
    assert_eq!(v["features"][0]["passes"], false);

    let commit_count = Command::new("git")
        .arg("-C")
        .arg(&workspace)
        .args(["rev-list", "--count", "HEAD"])
        .output()
        .unwrap();
    assert!(commit_count.status.success());
    assert_eq!(String::from_utf8_lossy(&commit_count.stdout).trim(), "2");

    server.abort();
}

#[tokio::test]
async fn harness_bootstrap_with_prompt_errors_if_initializer_does_not_write_features_json() {
    async fn handler(
        Json(_payload): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        Json(json!({
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "I would call fs_write to update features.json, but here is only a description."
                },
                "finish_reason": "stop"
            }]
        }))
    }

    let app = Router::new().route("/v1/chat/completions", post(handler));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("ws");

    let home = tmp.path().join("home");
    let paths = rexos::paths::RexosPaths {
        base_dir: home.join(".rexos"),
    };
    paths.ensure_dirs().unwrap();

    let memory = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();

    let mut providers = BTreeMap::new();
    providers.insert(
        "p".to_string(),
        rexos::config::ProviderConfig {
            kind: rexos::config::ProviderKind::OpenAiCompatible,
            base_url: format!("http://{addr}/v1"),
            api_key_env: "".to_string(),
            default_model: "x".to_string(),
        },
    );

    let cfg = rexos::config::RexosConfig {
        llm: rexos::config::LlmConfig::default(),
        providers,
        router: rexos::config::RouterConfig::default(),
    };
    let llms = rexos::llm::registry::LlmRegistry::from_config(&cfg).unwrap();
    let router = rexos::router::ModelRouter::new(rexos::config::RouterConfig {
        planning: rexos::config::RouteConfig {
            provider: "p".to_string(),
            model: "x".to_string(),
        },
        coding: rexos::config::RouteConfig {
            provider: "p".to_string(),
            model: "x".to_string(),
        },
        summary: rexos::config::RouteConfig {
            provider: "p".to_string(),
            model: "x".to_string(),
        },
    });

    let agent = rexos::agent::AgentRuntime::new(memory, llms, router);

    let err = rexos::harness::bootstrap_with_prompt(&agent, &workspace, "s1", "make a plan")
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("features.json"),
        "unexpected error: {err}"
    );

    server.abort();
}

#[tokio::test]
async fn harness_bootstrap_with_prompt_normalizes_minimal_features_json() {
    async fn handler(
        State(state): State<TestState>,
        Json(_payload): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        let mut calls = state.calls.lock().unwrap();
        *calls += 1;

        if *calls == 1 {
            let minimal = serde_json::json!({
                "features": [{
                    "id": "feat_1",
                    "description": "First feature",
                    "steps": ["do thing"]
                }]
            });
            return Json(json!({
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": null,
                        "tool_calls": [{
                            "id": "call_1",
                            "type": "function",
                            "function": {
                                "name": "fs_write",
                                "arguments": serde_json::to_string(&json!({
                                    "path": "features.json",
                                    "content": serde_json::to_string(&minimal).unwrap()
                                })).unwrap()
                            }
                        }]
                    },
                    "finish_reason": "tool_calls"
                }]
            }));
        }

        Json(json!({
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "ok" },
                "finish_reason": "stop"
            }]
        }))
    }

    let state = TestState::default();
    let app = Router::new()
        .route("/v1/chat/completions", post(handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("ws");

    let home = tmp.path().join("home");
    let paths = rexos::paths::RexosPaths {
        base_dir: home.join(".rexos"),
    };
    paths.ensure_dirs().unwrap();

    let memory = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();

    let mut providers = BTreeMap::new();
    providers.insert(
        "p".to_string(),
        rexos::config::ProviderConfig {
            kind: rexos::config::ProviderKind::OpenAiCompatible,
            base_url: format!("http://{addr}/v1"),
            api_key_env: "".to_string(),
            default_model: "x".to_string(),
        },
    );

    let cfg = rexos::config::RexosConfig {
        llm: rexos::config::LlmConfig::default(),
        providers,
        router: rexos::config::RouterConfig::default(),
    };
    let llms = rexos::llm::registry::LlmRegistry::from_config(&cfg).unwrap();
    let router = rexos::router::ModelRouter::new(rexos::config::RouterConfig {
        planning: rexos::config::RouteConfig {
            provider: "p".to_string(),
            model: "x".to_string(),
        },
        coding: rexos::config::RouteConfig {
            provider: "p".to_string(),
            model: "x".to_string(),
        },
        summary: rexos::config::RouteConfig {
            provider: "p".to_string(),
            model: "x".to_string(),
        },
    });

    let agent = rexos::agent::AgentRuntime::new(memory, llms, router);

    rexos::harness::bootstrap_with_prompt(&agent, &workspace, "s1", "make a plan")
        .await
        .unwrap();

    let raw = std::fs::read_to_string(workspace.join("features.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(v["version"], 1);
    assert!(v["rules"]["editing"].as_str().unwrap_or("").contains("passes"));
    assert_eq!(v["features"].as_array().unwrap().len(), 1);
    assert_eq!(v["features"][0]["passes"], false);

    server.abort();
}
