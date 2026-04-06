use std::sync::{Arc, Mutex, OnceLock};

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::json;

static ENV_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

fn env_lock() -> &'static tokio::sync::Mutex<()> {
    ENV_LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

#[derive(Clone, Default)]
struct TestState {
    captured: Arc<Mutex<Option<serde_json::Value>>>,
    calls: Arc<Mutex<u32>>,
}

struct EnvVarGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let previous = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(v) => std::env::set_var(self.key, v),
            None => std::env::remove_var(self.key),
        }
    }
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

    let client =
        rexos::llm::openai_compat::OpenAiCompatibleClient::new(format!("http://{addr}/v1"), None)
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

#[tokio::test]
async fn openai_compat_client_retries_transient_http_errors() {
    let _lock = env_lock().lock().await;
    let _guard = EnvVarGuard::set("LOOPFORGE_LLM_RETRY_MAX", "1");

    async fn handler(
        State(state): State<TestState>,
        Json(payload): Json<serde_json::Value>,
    ) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
        *state.captured.lock().unwrap() = Some(payload);
        let mut calls = state.calls.lock().unwrap();
        *calls += 1;

        if *calls == 1 {
            return Err(axum::http::StatusCode::SERVICE_UNAVAILABLE);
        }

        Ok(Json(json!({
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "ok" },
                "finish_reason": "stop"
            }]
        })))
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

    let client =
        rexos::llm::openai_compat::OpenAiCompatibleClient::new(format!("http://{addr}/v1"), None)
            .unwrap();

    let msg = rexos::llm::openai_compat::ChatMessage {
        role: rexos::llm::openai_compat::Role::User,
        content: Some("hello".to_string()),
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
    assert_eq!(res.content.as_deref(), Some("ok"));
    assert_eq!(*state.calls.lock().unwrap(), 2);

    server.abort();
}

#[tokio::test]
async fn openai_compat_client_parses_legacy_function_call_into_tool_call() {
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
                    "function_call": {
                        "name": "fs_read",
                        "arguments": "{\"path\":\"README.md\"}"
                    }
                },
                "finish_reason": "function_call"
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

    let client =
        rexos::llm::openai_compat::OpenAiCompatibleClient::new(format!("http://{addr}/v1"), None)
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

    server.abort();
}

#[tokio::test]
async fn openai_compat_client_timeout_can_be_overridden_via_env() {
    let _lock = env_lock().lock().await;
    let _guard = EnvVarGuard::set("LOOPFORGE_OPENAI_COMPAT_TIMEOUT_SECS", "1");
    let _retry_guard = EnvVarGuard::set("LOOPFORGE_LLM_RETRY_MAX", "0");

    async fn handler() -> Json<serde_json::Value> {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        Json(json!({
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "ok" },
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

    let client =
        rexos::llm::openai_compat::OpenAiCompatibleClient::new(format!("http://{addr}/v1"), None)
            .unwrap();

    let msg = rexos::llm::openai_compat::ChatMessage {
        role: rexos::llm::openai_compat::Role::User,
        content: Some("hello".to_string()),
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
        .await;

    assert!(res.is_err(), "expected request to time out");

    server.abort();
}
