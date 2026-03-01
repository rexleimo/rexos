use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::json;

#[derive(Clone, Default)]
struct TestState {
    calls: Arc<Mutex<u32>>,
    last_request: Arc<Mutex<Option<serde_json::Value>>>,
}

#[tokio::test]
async fn agent_loop_executes_tool_calls_and_persists_history() {
    async fn handler(
        State(state): State<TestState>,
        Json(payload): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        *state.last_request.lock().unwrap() = Some(payload);
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
                                "arguments": "{\"path\":\"hello.txt\",\"content\":\"hi\"}"
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
                "message": {
                    "role": "assistant",
                    "content": "done"
                },
                "finish_reason": "stop"
            }]
        }))
    }

    let state = TestState::default();
    let app = Router::new()
        .route("/v1/chat/completions", post(handler))
        .with_state(state.clone());

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
    let llm = rexos::llm::openai_compat::OpenAiCompatibleClient::new(
        format!("http://{addr}/v1"),
        None,
    )
    .unwrap();
    let router = rexos::router::ModelRouter::new(rexos::config::RouterConfig {
        planning_model: "x".to_string(),
        coding_model: "x".to_string(),
        summary_model: "x".to_string(),
    });

    let agent = rexos::agent::AgentRuntime::new(memory, llm, router);

    let out = agent
        .run_session(
            workspace.clone(),
            "s1",
            None,
            "write hello file",
            rexos::router::TaskKind::Coding,
        )
        .await
        .unwrap();
    assert_eq!(out, "done");

    assert_eq!(std::fs::read_to_string(workspace.join("hello.txt")).unwrap(), "hi");

    let memory2 = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();
    let msgs = memory2.list_chat_messages("s1").unwrap();
    assert!(msgs.len() >= 4);

    let last_req = state.last_request.lock().unwrap().clone().unwrap();
    let roles: Vec<String> = last_req["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|m| m.get("role").and_then(|r| r.as_str()).map(|s| s.to_string()))
        .collect();
    assert!(roles.contains(&"tool".to_string()));

    server.abort();
}
