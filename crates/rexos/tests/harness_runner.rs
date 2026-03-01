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
async fn harness_run_retries_on_init_sh_failure_and_checkpoints() {
    async fn handler(
        State(state): State<TestState>,
        Json(_payload): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        let mut calls = state.calls.lock().unwrap();
        *calls += 1;

        match *calls {
            1 => Json(json!({
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
                                    "path": "init.sh",
                                    "content": "#!/usr/bin/env bash\nexit 1\n"
                                })).unwrap()
                            }
                        }]
                    },
                    "finish_reason": "tool_calls"
                }]
            })),
            2 => Json(json!({
                "choices": [{
                    "index": 0,
                    "message": { "role": "assistant", "content": "attempt1" },
                    "finish_reason": "stop"
                }]
            })),
            3 => Json(json!({
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": null,
                        "tool_calls": [
                            {
                                "id": "call_2",
                                "type": "function",
                                "function": {
                                    "name": "fs_write",
                                    "arguments": serde_json::to_string(&json!({
                                        "path": "init.sh",
                                        "content": "#!/usr/bin/env bash\nexit 0\n"
                                    })).unwrap()
                                }
                            },
                            {
                                "id": "call_3",
                                "type": "function",
                                "function": {
                                    "name": "fs_write",
                                    "arguments": serde_json::to_string(&json!({
                                        "path": "marker.txt",
                                        "content": "ok"
                                    })).unwrap()
                                }
                            }
                        ]
                    },
                    "finish_reason": "tool_calls"
                }]
            })),
            _ => Json(json!({
                "choices": [{
                    "index": 0,
                    "message": { "role": "assistant", "content": "attempt2" },
                    "finish_reason": "stop"
                }]
            })),
        }
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
    rexos::harness::init_workspace(&workspace).unwrap();

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

    let out = rexos::harness::run_harness(&agent, &workspace, "s1", "do it", 3)
        .await
        .unwrap();
    assert!(!out.trim().is_empty());
    assert!(workspace.join("marker.txt").exists());

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

