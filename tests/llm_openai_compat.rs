use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::json;

#[derive(Clone, Default)]
struct TestState {
    captured: Arc<Mutex<Option<serde_json::Value>>>,
}

#[tokio::test]
async fn openai_compat_client_posts_and_parses_tool_calls() {
    async fn handler(
        State(state): State<TestState>,
        Json(payload): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        *state.captured.lock().unwrap() = Some(payload);
        Json(json!({
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": { "name": "fs_read", "arguments": "{\"path\":\"README.md\"}" }
                    }]
                },
                "finish_reason": "tool_calls"
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

    let client = rexos::llm::openai_compat::OpenAiCompatibleClient::new(
        format!("http://{addr}/v1"),
        None,
    )
    .unwrap();

    let msg = rexos::llm::openai_compat::ChatMessage {
        role: rexos::llm::openai_compat::Role::User,
        content: Some("read file".to_string()),
        name: None,
        tool_call_id: None,
        tool_calls: None,
    };

    let res = client
        .chat_completions(rexos::llm::openai_compat::ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![msg],
            tools: vec![],
            temperature: None,
        })
        .await
        .unwrap();

    assert_eq!(res.role, rexos::llm::openai_compat::Role::Assistant);
    assert_eq!(res.content, None);
    let calls = res.tool_calls.unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].id, "call_1");
    assert_eq!(calls[0].function.name, "fs_read");
    assert_eq!(calls[0].function.arguments, "{\"path\":\"README.md\"}");

    let captured = state.captured.lock().unwrap().clone().unwrap();
    assert_eq!(captured["model"], "test-model");
    assert_eq!(captured["messages"][0]["role"], "user");
    assert_eq!(captured["messages"][0]["content"], "read file");

    server.abort();
}

