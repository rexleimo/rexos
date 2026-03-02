use std::collections::BTreeMap;
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
async fn reserved_cron_tools_create_list_and_cancel() {
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
                        "tool_calls": [
                            {
                                "id": "call_1",
                                "type": "function",
                                "function": {
                                    "name": "cron_create",
                                    "arguments": serde_json::to_string(&json!({
                                        "job_id": "job1",
                                        "name": "Job One",
                                        "schedule": { "kind": "every", "every_secs": 60 },
                                        "action": { "kind": "system_event", "text": "ping" },
                                        "one_shot": false
                                    })).unwrap()
                                }
                            },
                            {
                                "id": "call_2",
                                "type": "function",
                                "function": { "name": "cron_list", "arguments": "{}" }
                            },
                            {
                                "id": "call_3",
                                "type": "function",
                                "function": {
                                    "name": "cron_cancel",
                                    "arguments": serde_json::to_string(&json!({ "job_id": "job1" })).unwrap()
                                }
                            },
                            {
                                "id": "call_4",
                                "type": "function",
                                "function": { "name": "cron_list", "arguments": "{}" }
                            }
                        ]
                    },
                    "finish_reason": "tool_calls"
                }]
            }));
        }

        Json(json!({
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "done" },
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
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();

    let home = tmp.path().join("home");
    let paths = rexos::paths::RexosPaths {
        base_dir: home.join(".rexos"),
    };
    paths.ensure_dirs().unwrap();

    let memory = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();

    let mut providers = BTreeMap::new();
    providers.insert(
        "mock".to_string(),
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
            provider: "mock".to_string(),
            model: "x".to_string(),
        },
        coding: rexos::config::RouteConfig {
            provider: "mock".to_string(),
            model: "x".to_string(),
        },
        summary: rexos::config::RouteConfig {
            provider: "mock".to_string(),
            model: "x".to_string(),
        },
    });

    let agent = rexos::agent::AgentRuntime::new(memory, llms, router);

    let out = agent
        .run_session(
            workspace,
            "s1",
            None,
            "exercise reserved cron tools",
            rexos::router::TaskKind::Coding,
        )
        .await
        .unwrap();
    assert_eq!(out, "done");

    let memory2 = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();
    let msgs = memory2.list_chat_messages("s1").unwrap();
    let cron_lists: Vec<String> = msgs
        .iter()
        .filter(|m| {
            m.role == rexos::llm::openai_compat::Role::Tool && m.name.as_deref() == Some("cron_list")
        })
        .filter_map(|m| m.content.clone())
        .collect();
    assert_eq!(cron_lists.len(), 2, "cron_list outputs: {cron_lists:?}");

    let v1: serde_json::Value =
        serde_json::from_str(&cron_lists[0]).expect("cron_list[0] should be json");
    assert_eq!(v1.as_array().map(|a| a.len()), Some(1), "{v1}");
    assert_eq!(
        v1[0].get("job_id").and_then(|v| v.as_str()),
        Some("job1"),
        "{v1}"
    );

    let v2: serde_json::Value =
        serde_json::from_str(&cron_lists[1]).expect("cron_list[1] should be json");
    assert_eq!(v2.as_array().map(|a| a.len()), Some(0), "{v2}");

    server.abort();
}

