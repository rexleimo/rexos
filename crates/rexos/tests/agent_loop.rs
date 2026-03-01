use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::json;
use std::collections::BTreeMap;

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
    let mut providers = BTreeMap::new();
    providers.insert(
        "ollama".to_string(),
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

    let agent = rexos::agent::AgentRuntime::new(memory, llms, router);

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
    assert_eq!(
        last_req.get("temperature").and_then(|v| v.as_f64()),
        Some(0.0)
    );

    server.abort();
}

#[tokio::test]
async fn agent_loop_should_route_by_provider_name() {
    async fn handler_p1() -> Json<serde_json::Value> {
        Json(json!({
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "p1" },
                "finish_reason": "stop"
            }]
        }))
    }

    async fn handler_p2() -> Json<serde_json::Value> {
        Json(json!({
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "p2" },
                "finish_reason": "stop"
            }]
        }))
    }

    let app1 = Router::new().route("/v1/chat/completions", post(handler_p1));
    let listener1 = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr1 = listener1.local_addr().unwrap();
    let server1 = tokio::spawn(async move {
        axum::serve(listener1, app1).await.unwrap();
    });

    let app2 = Router::new().route("/v1/chat/completions", post(handler_p2));
    let listener2 = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr2 = listener2.local_addr().unwrap();
    let server2 = tokio::spawn(async move {
        axum::serve(listener2, app2).await.unwrap();
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
        "p1".to_string(),
        rexos::config::ProviderConfig {
            kind: rexos::config::ProviderKind::OpenAiCompatible,
            base_url: format!("http://{addr1}/v1"),
            api_key_env: "".to_string(),
            default_model: "x".to_string(),
        },
    );
    providers.insert(
        "p2".to_string(),
        rexos::config::ProviderConfig {
            kind: rexos::config::ProviderKind::OpenAiCompatible,
            base_url: format!("http://{addr2}/v1"),
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
            provider: "p1".to_string(),
            model: "x".to_string(),
        },
        coding: rexos::config::RouteConfig {
            provider: "p1".to_string(),
            model: "x".to_string(),
        },
        summary: rexos::config::RouteConfig {
            provider: "p1".to_string(),
            model: "x".to_string(),
        },
    });

    let agent = rexos::agent::AgentRuntime::new(memory, llms, router);

    let out = agent
        .run_session(
            workspace,
            "s-route",
            None,
            "hello",
            rexos::router::TaskKind::Coding,
        )
        .await
        .unwrap();

    assert_eq!(out, "p1");

    server1.abort();
    server2.abort();
}

#[tokio::test]
async fn agent_loop_executes_tool_calls_from_json_content_fallback() {
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
                        "content": "[{\"name\":\"fs_write\",\"arguments\":{\"path\":\"hello.txt\",\"content\":\"hi\"}}]"
                    },
                    "finish_reason": "stop"
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
    let mut providers = BTreeMap::new();
    providers.insert(
        "ollama".to_string(),
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

    let agent = rexos::agent::AgentRuntime::new(memory, llms, router);

    let out = agent
        .run_session(
            workspace.clone(),
            "s-json-fallback",
            None,
            "write hello file",
            rexos::router::TaskKind::Coding,
        )
        .await
        .unwrap();
    assert_eq!(out, "done");

    assert_eq!(std::fs::read_to_string(workspace.join("hello.txt")).unwrap(), "hi");

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

#[tokio::test]
async fn agent_loop_unwraps_wrapped_tool_arguments() {
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
                                "arguments": "{\"function\":\"fs_write\",\"arguments\":{\"path\":\"hello.txt\",\"content\":\"hi\"}}"
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
    let mut providers = BTreeMap::new();
    providers.insert(
        "ollama".to_string(),
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

    let agent = rexos::agent::AgentRuntime::new(memory, llms, router);

    let out = agent
        .run_session(
            workspace.clone(),
            "s-unwrap-args",
            None,
            "write hello file",
            rexos::router::TaskKind::Coding,
        )
        .await
        .unwrap();
    assert_eq!(out, "done");

    assert_eq!(std::fs::read_to_string(workspace.join("hello.txt")).unwrap(), "hi");

    server.abort();
}

#[tokio::test]
async fn agent_loop_executes_tool_calls_from_embedded_json_snippets() {
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
                        "content": "Call this tool:\n{\"name\":\"fs_write\",\"arguments\":{\"path\":\"hello.txt\",\"content\":\"hi\"}}"
                    },
                    "finish_reason": "stop"
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
    let mut providers = BTreeMap::new();
    providers.insert(
        "ollama".to_string(),
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

    let agent = rexos::agent::AgentRuntime::new(memory, llms, router);

    let out = agent
        .run_session(
            workspace.clone(),
            "s-json-snippets",
            None,
            "write hello file",
            rexos::router::TaskKind::Coding,
        )
        .await
        .unwrap();
    assert_eq!(out, "done");

    assert_eq!(std::fs::read_to_string(workspace.join("hello.txt")).unwrap(), "hi");

    server.abort();
}

#[tokio::test]
async fn agent_loop_executes_tool_calls_from_tagged_tool_call_json() {
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
                        "content": "[tool_call]\n{\n  \"name\": \"fs_write\",\n  \"args\": {\"path\": \"hello.txt\", \"content\": \"hi\"}\n}"
                    },
                    "finish_reason": "stop"
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
    let mut providers = BTreeMap::new();
    providers.insert(
        "ollama".to_string(),
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

    let agent = rexos::agent::AgentRuntime::new(memory, llms, router);

    let out = agent
        .run_session(
            workspace.clone(),
            "s-tagged-tool-call",
            None,
            "write hello file",
            rexos::router::TaskKind::Coding,
        )
        .await
        .unwrap();
    assert_eq!(out, "done");

    assert_eq!(std::fs::read_to_string(workspace.join("hello.txt")).unwrap(), "hi");

    server.abort();
}

#[tokio::test]
async fn agent_loop_executes_tool_calls_from_flattened_call_objects() {
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
                        "content": "[{\"name\":\"fs_write\",\"path\":\"hello.txt\",\"content\":\"hi\"}]"
                    },
                    "finish_reason": "stop"
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
    let mut providers = BTreeMap::new();
    providers.insert(
        "ollama".to_string(),
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

    let agent = rexos::agent::AgentRuntime::new(memory, llms, router);

    let out = agent
        .run_session(
            workspace.clone(),
            "s-flat-call",
            None,
            "write hello file",
            rexos::router::TaskKind::Coding,
        )
        .await
        .unwrap();
    assert_eq!(out, "done");

    assert_eq!(std::fs::read_to_string(workspace.join("hello.txt")).unwrap(), "hi");

    server.abort();
}
