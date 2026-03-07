use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::json;

#[derive(Clone, Default)]
struct TestState {
    calls: Arc<Mutex<u32>>,
    payloads: Arc<Mutex<Vec<serde_json::Value>>>,
}

struct EnvVarGuard {
    key: &'static str,
    previous: Option<String>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(v) = self.previous.take() {
            std::env::set_var(self.key, v);
        } else {
            std::env::remove_var(self.key);
        }
    }
}

fn test_agent(base_url: String, memory: rexos::memory::MemoryStore) -> rexos::agent::AgentRuntime {
    test_agent_with_security(base_url, memory, rexos::security::SecurityConfig::default())
}

fn test_agent_with_security(
    base_url: String,
    memory: rexos::memory::MemoryStore,
    security: rexos::security::SecurityConfig,
) -> rexos::agent::AgentRuntime {
    let mut providers = BTreeMap::new();
    providers.insert(
        "ollama".to_string(),
        rexos::config::ProviderConfig {
            kind: rexos::config::ProviderKind::OpenAiCompatible,
            base_url,
            api_key_env: "".to_string(),
            default_model: "x".to_string(),
        },
    );

    let cfg = rexos::config::RexosConfig {
        llm: rexos::config::LlmConfig::default(),
        providers,
        router: rexos::config::RouterConfig::default(),
        security: security.clone(),
    };
    let llms = rexos::llm::registry::LlmRegistry::from_config(&cfg).unwrap();
    let router = rexos::router::ModelRouter::new(rexos::config::RouterConfig {
        planning: rexos::config::RouteConfig {
            provider: "ollama".to_string(),
            model: "x".to_string(),
        },
        coding: rexos::config::RouteConfig {
            provider: "ollama".to_string(),
            model: "x".to_string(),
        },
        summary: rexos::config::RouteConfig {
            provider: "ollama".to_string(),
            model: "x".to_string(),
        },
    });
    rexos::agent::AgentRuntime::new_with_security_config(memory, llms, router, security)
}

#[tokio::test]
async fn session_tool_whitelist_blocks_tool_and_audits_failure() {
    async fn handler(
        State(state): State<TestState>,
        Json(_payload): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        let mut calls = state.calls.lock().unwrap();
        *calls += 1;

        Json(json!({
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
                            "arguments": "{\"path\":\"x.txt\",\"content\":\"blocked\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
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
    let paths = rexos::paths::RexosPaths {
        base_dir: tmp.path().join(".loopforge"),
    };
    paths.ensure_dirs().unwrap();
    let memory = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();

    let agent = test_agent(format!("http://{addr}/v1"), memory);
    agent
        .set_session_allowed_tools("s-whitelist", vec!["fs_read".to_string()])
        .unwrap();

    let err = agent
        .run_session(
            workspace.clone(),
            "s-whitelist",
            None,
            "try write",
            rexos::router::TaskKind::Coding,
        )
        .await
        .unwrap_err();
    let err_text = err.to_string();
    assert!(
        err_text.contains("tool not allowed"),
        "expected tool deny error, got: {err_text}"
    );

    let memory2 = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();
    let raw = memory2
        .kv_get("rexos.audit.tool_calls")
        .unwrap()
        .unwrap_or_default();
    let events: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let arr = events.as_array().unwrap();
    let last = arr.last().unwrap();
    assert_eq!(last["session_id"], "s-whitelist");
    assert_eq!(last["tool_name"], "fs_write");
    assert_eq!(last["success"], false);
    assert!(last["error"]
        .as_str()
        .unwrap_or("")
        .contains("tool not allowed"));

    server.abort();
}

#[tokio::test]
async fn tool_audit_marks_truncated_for_large_output() {
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
                                "name": "fs_read",
                                "arguments": "{\"path\":\"large.txt\"}"
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
    std::fs::write(
        workspace.join("large.txt"),
        format!("HEAD:{}:TAIL", "x".repeat(25_000)),
    )
    .unwrap();

    let paths = rexos::paths::RexosPaths {
        base_dir: tmp.path().join(".loopforge"),
    };
    paths.ensure_dirs().unwrap();
    let memory = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();
    let agent = test_agent(format!("http://{addr}/v1"), memory);

    let out = agent
        .run_session(
            workspace,
            "s-truncate-audit",
            None,
            "read large file",
            rexos::router::TaskKind::Coding,
        )
        .await
        .unwrap();
    assert_eq!(out, "done");

    let memory2 = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();
    let raw = memory2
        .kv_get("rexos.audit.tool_calls")
        .unwrap()
        .unwrap_or_default();
    let events: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let arr = events.as_array().unwrap();
    let event = arr
        .iter()
        .rev()
        .find(|v| v["session_id"] == "s-truncate-audit")
        .expect("expected audit event");
    assert_eq!(event["tool_name"], "fs_read");
    assert_eq!(event["success"], true);
    assert_eq!(event["truncated"], true);
    assert!(event["duration_ms"].as_u64().is_some());

    server.abort();
}

#[tokio::test]
async fn leak_guard_redacts_tool_output_before_model_and_audit_persistence() {
    async fn handler(
        State(state): State<TestState>,
        Json(payload): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        state.payloads.lock().unwrap().push(payload);

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
                                "name": "fs_read",
                                "arguments": "{\"path\":\"secret.txt\"}"
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
                "message": { "role": "assistant", "content": "done" },
                "finish_reason": "stop"
            }]
        }))
    }

    let _guard = EnvVarGuard::set("LOOPFORGE_TEST_SECRET_REDACT_RT", "super-secret-redact-rt-value-12345");

    let state = TestState::default();
    let state_for_asserts = state.clone();
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
    std::fs::write(workspace.join("secret.txt"), "super-secret-redact-rt-value-12345").unwrap();

    let paths = rexos::paths::RexosPaths {
        base_dir: tmp.path().join(".loopforge"),
    };
    paths.ensure_dirs().unwrap();
    let memory = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();

    let mut security = rexos::security::SecurityConfig::default();
    security.leaks.mode = rexos::security::LeakMode::Redact;
    let agent = test_agent_with_security(format!("http://{addr}/v1"), memory, security);
    std::env::remove_var("LOOPFORGE_TEST_SECRET_REDACT_RT");

    let out = agent
        .run_session(
            workspace,
            "s-leak-redact",
            None,
            "read secret",
            rexos::router::TaskKind::Coding,
        )
        .await
        .unwrap();
    assert_eq!(out, "done");

    let payloads = state_for_asserts.payloads.lock().unwrap().clone();
    assert!(
        payloads.len() >= 2,
        "expected two model calls, got {payloads:?}"
    );
    let second_payload = serde_json::to_string(&payloads[1]).unwrap();
    assert!(
        !second_payload.contains("super-secret-redact-rt-value-12345"),
        "expected redacted model payload, got: {second_payload}"
    );
    assert!(
        second_payload.contains("[redacted:env:LOOPFORGE_TEST_SECRET_REDACT_RT]"),
        "expected redaction marker in model payload, got: {second_payload}"
    );

    let memory2 = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();
    let messages = memory2.list_chat_messages("s-leak-redact").unwrap();
    let tool_message = messages
        .iter()
        .find(|msg| msg.role == rexos::llm::openai_compat::Role::Tool)
        .expect("expected tool message");
    let tool_content = tool_message.content.as_deref().unwrap_or("");
    assert!(
        !tool_content.contains("super-secret-redact-rt-value-12345"),
        "{tool_content}"
    );
    assert!(
        tool_content.contains("[redacted:env:LOOPFORGE_TEST_SECRET_REDACT_RT]"),
        "{tool_content}"
    );

    let raw = memory2
        .kv_get("rexos.audit.tool_calls")
        .unwrap()
        .unwrap_or_default();
    assert!(
        !raw.contains("super-secret-redact-rt-value-12345"),
        "tool audit leaked raw secret: {raw}"
    );
    let events: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let arr = events.as_array().unwrap();
    let event = arr
        .iter()
        .rev()
        .find(|v| v["session_id"] == "s-leak-redact")
        .expect("expected audit event");
    assert_eq!(event["tool_name"], "fs_read");
    assert_eq!(event["success"], true);
    assert_eq!(event["leak_guard"]["mode"], "redact");
    assert_eq!(event["leak_guard"]["redacted"], true);

    server.abort();
}

#[tokio::test]
async fn leak_guard_enforce_blocks_tool_output_without_persisting_secret() {
    async fn handler(
        State(state): State<TestState>,
        Json(payload): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        state.payloads.lock().unwrap().push(payload);

        let mut calls = state.calls.lock().unwrap();
        *calls += 1;

        Json(json!({
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "fs_read",
                            "arguments": "{\"path\":\"secret.txt\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        }))
    }

    let _guard = EnvVarGuard::set("LOOPFORGE_TEST_SECRET_ENFORCE_RT", "super-secret-enforce-rt-value-12345");

    let state = TestState::default();
    let state_for_asserts = state.clone();
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
    std::fs::write(workspace.join("secret.txt"), "super-secret-enforce-rt-value-12345").unwrap();

    let paths = rexos::paths::RexosPaths {
        base_dir: tmp.path().join(".loopforge"),
    };
    paths.ensure_dirs().unwrap();
    let memory = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();

    let mut security = rexos::security::SecurityConfig::default();
    security.leaks.mode = rexos::security::LeakMode::Enforce;
    let agent = test_agent_with_security(format!("http://{addr}/v1"), memory, security);
    std::env::remove_var("LOOPFORGE_TEST_SECRET_ENFORCE_RT");

    let err = agent
        .run_session(
            workspace,
            "s-leak-enforce",
            None,
            "read secret",
            rexos::router::TaskKind::Coding,
        )
        .await
        .unwrap_err();
    assert_eq!(err.to_string(), "tool output blocked by leak guard");

    let payloads = state_for_asserts.payloads.lock().unwrap().clone();
    assert_eq!(
        payloads.len(),
        1,
        "unexpected extra model calls: {payloads:?}"
    );

    let memory2 = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();
    let audit_raw = memory2
        .kv_get("rexos.audit.tool_calls")
        .unwrap()
        .unwrap_or_default();
    assert!(
        !audit_raw.contains("super-secret-enforce-rt-value-12345"),
        "tool audit leaked raw secret: {audit_raw}"
    );
    let events: serde_json::Value = serde_json::from_str(&audit_raw).unwrap();
    let arr = events.as_array().unwrap();
    let event = arr
        .iter()
        .rev()
        .find(|v| v["session_id"] == "s-leak-enforce")
        .expect("expected audit event");
    assert_eq!(event["tool_name"], "fs_read");
    assert_eq!(event["success"], false);
    assert_eq!(event["error"], "tool output blocked by leak guard");
    assert_eq!(event["leak_guard"]["mode"], "enforce");
    assert_eq!(event["leak_guard"]["blocked"], true);

    let acp_raw = memory2
        .kv_get("rexos.acp.events")
        .unwrap()
        .unwrap_or_default();
    assert!(
        !acp_raw.contains("super-secret-enforce-rt-value-12345"),
        "acp events leaked raw secret: {acp_raw}"
    );
    assert!(
        acp_raw.contains("tool.blocked") || acp_raw.contains("tool.failed"),
        "{acp_raw}"
    );

    server.abort();
}

#[tokio::test]
async fn acp_events_capture_session_and_tool_lifecycle() {
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
                                "name": "fs_read",
                                "arguments": "{\"path\":\"hello.txt\"}"
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
    std::fs::write(workspace.join("hello.txt"), "hello").unwrap();

    let paths = rexos::paths::RexosPaths {
        base_dir: tmp.path().join(".loopforge"),
    };
    paths.ensure_dirs().unwrap();
    let memory = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();
    let agent = test_agent(format!("http://{addr}/v1"), memory);

    let out = agent
        .run_session(
            workspace,
            "s-acp-events",
            None,
            "read file",
            rexos::router::TaskKind::Coding,
        )
        .await
        .unwrap();
    assert_eq!(out, "done");

    let memory2 = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();
    let raw = memory2
        .kv_get("rexos.acp.events")
        .unwrap()
        .unwrap_or_default();
    let events: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let arr = events.as_array().unwrap();
    assert!(
        arr.iter()
            .any(|v| v["session_id"] == "s-acp-events" && v["event_type"] == "session.started"),
        "missing session.started: {events}"
    );
    assert!(
        arr.iter()
            .any(|v| v["session_id"] == "s-acp-events" && v["event_type"] == "tool.succeeded"),
        "missing tool.succeeded: {events}"
    );
    assert!(
        arr.iter()
            .any(|v| v["session_id"] == "s-acp-events" && v["event_type"] == "session.completed"),
        "missing session.completed: {events}"
    );

    server.abort();
}

#[tokio::test]
async fn approval_enforce_blocks_dangerous_tool_calls() {
    async fn handler(
        State(state): State<TestState>,
        Json(_payload): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        let mut calls = state.calls.lock().unwrap();
        *calls += 1;
        Json(json!({
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "shell",
                            "arguments": "{\"command\":\"echo hi\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        }))
    }

    let _guard_mode = EnvVarGuard::set("LOOPFORGE_APPROVAL_MODE", "enforce");
    let _guard_allow = EnvVarGuard::set("LOOPFORGE_APPROVAL_ALLOW", "");

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
    let paths = rexos::paths::RexosPaths {
        base_dir: tmp.path().join(".loopforge"),
    };
    paths.ensure_dirs().unwrap();
    let memory = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();
    let agent = test_agent(format!("http://{addr}/v1"), memory);

    let err = agent
        .run_session(
            workspace,
            "s-approval-enforce",
            None,
            "try shell",
            rexos::router::TaskKind::Coding,
        )
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("approval required"), "{msg}");

    let memory2 = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();
    let raw = memory2
        .kv_get("rexos.acp.events")
        .unwrap()
        .unwrap_or_default();
    let events: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let arr = events.as_array().expect("acp events should be an array");
    assert!(
        arr.iter()
            .any(|v| v["session_id"] == "s-approval-enforce"
                && v["event_type"] == "approval.blocked"),
        "missing approval.blocked event: {events}"
    );

    server.abort();
}

#[tokio::test]
async fn workflow_run_persists_state_and_executes_steps() {
    async fn handler(
        State(state): State<TestState>,
        Json(_payload): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        let mut calls = state.calls.lock().unwrap();
        *calls += 1;
        if *calls == 1 {
            let args = json!({
                "workflow_id": "wf-demo",
                "name": "demo",
                "steps": [
                    {
                        "name": "write note",
                        "tool": "fs_write",
                        "arguments": { "path": "workflow-note.txt", "content": "hello workflow" }
                    }
                ]
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
                                "name": "workflow_run",
                                "arguments": serde_json::to_string(&args).unwrap()
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
    let paths = rexos::paths::RexosPaths {
        base_dir: tmp.path().join(".loopforge"),
    };
    paths.ensure_dirs().unwrap();
    let memory = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();
    let agent = test_agent(format!("http://{addr}/v1"), memory);

    let out = agent
        .run_session(
            workspace.clone(),
            "s-workflow",
            None,
            "run workflow",
            rexos::router::TaskKind::Coding,
        )
        .await
        .unwrap();
    assert_eq!(out, "done");
    assert_eq!(
        std::fs::read_to_string(workspace.join("workflow-note.txt")).unwrap(),
        "hello workflow"
    );

    let state_path = workspace.join(".loopforge/workflows/wf-demo.json");
    let state_raw = std::fs::read_to_string(&state_path).unwrap();
    let state_json: serde_json::Value = serde_json::from_str(&state_raw).unwrap();
    assert_eq!(state_json["status"], "completed");
    assert_eq!(state_json["steps"][0]["status"], "succeeded");

    server.abort();
}

#[tokio::test]
async fn delivery_checkpoint_is_written_after_dispatch() {
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
                                "name": "channel_send",
                                "arguments": "{\"channel\":\"console\",\"recipient\":\"user1\",\"message\":\"hi\"}"
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
    let paths = rexos::paths::RexosPaths {
        base_dir: tmp.path().join(".loopforge"),
    };
    paths.ensure_dirs().unwrap();
    let memory = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();
    let agent = test_agent(format!("http://{addr}/v1"), memory);

    let out = agent
        .run_session(
            workspace,
            "s-checkpoint",
            None,
            "send",
            rexos::router::TaskKind::Coding,
        )
        .await
        .unwrap();
    assert_eq!(out, "done");

    let dispatcher = rexos::agent::OutboxDispatcher::new(
        rexos::memory::MemoryStore::open_or_create(&paths).unwrap(),
    )
    .unwrap();
    let summary = dispatcher.drain_once(10).await.unwrap();
    assert_eq!(summary.sent, 1);

    let memory2 = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();
    let raw = memory2
        .kv_get("rexos.acp.checkpoints.s-checkpoint")
        .unwrap()
        .unwrap_or_default();
    let checkpoints: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let arr = checkpoints
        .as_array()
        .expect("acp checkpoints should be an array");
    assert!(
        arr.iter().any(|v| v["channel"] == "console"),
        "missing console checkpoint: {checkpoints}"
    );

    server.abort();
}
