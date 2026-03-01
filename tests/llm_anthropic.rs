use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::json;

use rexos::llm::driver::LlmDriver;

#[derive(Clone, Default)]
struct TestState {
    captured: Arc<Mutex<Option<serde_json::Value>>>,
}

#[tokio::test]
async fn anthropic_driver_maps_system_tools_and_tool_use() {
    async fn handler(
        State(state): State<TestState>,
        Json(payload): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        *state.captured.lock().unwrap() = Some(payload);
        Json(json!({
            "content": [
                { "type": "tool_use", "id": "call_1", "name": "fs_read", "input": { "path": "README.md" } }
            ]
        }))
    }

    let state = TestState::default();
    let app = Router::new()
        .route("/v1/messages", post(handler))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let driver = rexos::llm::anthropic::AnthropicDriver::new(format!("http://{addr}"), Some("k".to_string())).unwrap();

    let req = rexos::llm::openai_compat::ChatCompletionRequest {
        model: "claude-test".to_string(),
        messages: vec![
            rexos::llm::openai_compat::ChatMessage {
                role: rexos::llm::openai_compat::Role::System,
                content: Some("sys".to_string()),
                name: None,
                tool_call_id: None,
                tool_calls: None,
            },
            rexos::llm::openai_compat::ChatMessage {
                role: rexos::llm::openai_compat::Role::User,
                content: Some("read it".to_string()),
                name: None,
                tool_call_id: None,
                tool_calls: None,
            },
        ],
        tools: vec![rexos::llm::openai_compat::ToolDefinition {
            kind: "function".to_string(),
            function: rexos::llm::openai_compat::ToolFunctionDefinition {
                name: "fs_read".to_string(),
                description: "Read file".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": { "path": { "type": "string" } },
                    "required": ["path"],
                    "additionalProperties": false
                }),
            },
        }],
        temperature: None,
    };

    let msg = driver.chat(req).await.unwrap();
    assert_eq!(msg.role, rexos::llm::openai_compat::Role::Assistant);
    let calls = msg.tool_calls.unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].function.name, "fs_read");

    let captured = state.captured.lock().unwrap().clone().unwrap();
    assert_eq!(captured["model"], "claude-test");
    assert_eq!(captured["system"], "sys");
    assert_eq!(captured["tools"][0]["name"], "fs_read");

    server.abort();
}
