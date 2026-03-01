use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::extract::{Query, State};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::json;

use rexos::llm::driver::LlmDriver;

#[derive(Clone, Default)]
struct TestState {
    captured: Arc<Mutex<Option<serde_json::Value>>>,
}

#[tokio::test]
async fn gemini_driver_maps_system_tools_and_function_call() {
    async fn handler(
        State(state): State<TestState>,
        Query(q): Query<HashMap<String, String>>,
        Json(payload): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        assert_eq!(q.get("key").map(String::as_str), Some("k"));
        *state.captured.lock().unwrap() = Some(payload);
        Json(json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{
                        "functionCall": { "name": "fs_read", "args": { "path": "README.md" } }
                    }]
                }
            }]
        }))
    }

    let state = TestState::default();
    let app = Router::new()
        .route("/v1beta/models/gemini-test:generateContent", post(handler))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let driver =
        rexos::llm::gemini::GeminiDriver::new(format!("http://{addr}/v1beta"), Some("k".to_string()))
            .unwrap();

    let req = rexos::llm::openai_compat::ChatCompletionRequest {
        model: "gemini-test".to_string(),
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
    assert!(captured.get("systemInstruction").is_some());
    assert_eq!(
        captured["tools"][0]["functionDeclarations"][0]["name"],
        "fs_read"
    );

    server.abort();
}

