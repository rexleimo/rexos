use std::collections::BTreeMap;

use rexos::config::{ProviderConfig, ProviderKind, RexosConfig, RouteConfig, RouterConfig};
use rexos::paths::RexosPaths;
use rexos::router::TaskKind;
use rexos::security::{EgressConfig, EgressRule, LeakMode, SecurityConfig};
use serde_json::{json, Value};
use serial_test::serial;

mod support;

struct EnvVarGuard {
    key: &'static str,
    prev: Option<String>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let prev = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, prev }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match self.prev.as_ref() {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}

fn fixture_agent(
    tmp: &tempfile::TempDir,
    fixture_base_url: String,
    security: SecurityConfig,
) -> (rexos::agent::AgentRuntime, RexosPaths, std::path::PathBuf) {
    let paths = RexosPaths {
        base_dir: tmp.path().join(".loopforge"),
    };
    paths.ensure_dirs().unwrap();

    let workspace_root = tmp.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).unwrap();

    let mut providers = BTreeMap::new();
    providers.insert(
        "fixture".to_string(),
        ProviderConfig {
            kind: ProviderKind::OpenAiCompatible,
            base_url: fixture_base_url,
            api_key_env: String::new(),
            default_model: "fixture-model".to_string(),
            aws_bedrock: None,
        },
    );

    let cfg = RexosConfig {
        llm: Default::default(),
        providers,
        router: RouterConfig {
            planning: RouteConfig {
                provider: "fixture".to_string(),
                model: "fixture-model".to_string(),
            },
            coding: RouteConfig {
                provider: "fixture".to_string(),
                model: "fixture-model".to_string(),
            },
            summary: RouteConfig {
                provider: "fixture".to_string(),
                model: "fixture-model".to_string(),
            },
        },
        security: security.clone(),
    };

    let llms = rexos::llm::registry::LlmRegistry::from_config(&cfg).unwrap();
    let router = rexos::router::ModelRouter::new(cfg.router);
    let memory = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();
    let agent =
        rexos::agent::AgentRuntime::new_with_security_config(memory, llms, router, security);

    (agent, paths, workspace_root)
}

fn compact_request(req: &Value) -> Value {
    fn sorted_string_array(value: &Value) -> Vec<String> {
        let mut out: Vec<String> = value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        out.sort();
        out
    }

    fn sorted_object_keys(value: &Value) -> Vec<String> {
        let mut out: Vec<String> = value
            .as_object()
            .into_iter()
            .flatten()
            .map(|(k, _)| k.to_string())
            .collect();
        out.sort();
        out
    }

    fn tool_schema_snapshot(tool: &Value) -> Value {
        let name = tool
            .get("function")
            .and_then(|f| f.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("<missing>");
        let params = tool
            .get("function")
            .and_then(|f| f.get("parameters"))
            .unwrap_or(&Value::Null);

        json!({
            "name": name,
            "type": tool.get("type").and_then(|v| v.as_str()).unwrap_or("<missing>"),
            "param_type": params.get("type").and_then(|v| v.as_str()).unwrap_or("<missing>"),
            "required": sorted_string_array(params.get("required").unwrap_or(&Value::Null)),
            "properties": sorted_object_keys(params.get("properties").unwrap_or(&Value::Null)),
            "additional_properties": params.get("additionalProperties").cloned().unwrap_or(Value::Null),
        })
    }

    let tools: Vec<Value> = req
        .get("tools")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .map(tool_schema_snapshot)
        .collect();
    let mut tools = tools;
    tools.sort_by(|a, b| {
        a["name"]
            .as_str()
            .unwrap_or("")
            .cmp(b["name"].as_str().unwrap_or(""))
    });

    let messages: Vec<&Value> = req
        .get("messages")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .collect();

    let message_roles: Vec<String> = messages
        .iter()
        .filter_map(|m| {
            m.get("role")
                .and_then(|r| r.as_str())
                .map(|s| s.to_string())
        })
        .collect();

    let mut assistant_tool_calls: Vec<Value> = Vec::new();
    let mut tool_messages: Vec<Value> = Vec::new();

    for msg in messages {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
        if role == "assistant" {
            let calls = msg.get("tool_calls").and_then(|v| v.as_array());
            for call in calls.into_iter().flatten() {
                let args_raw = call
                    .get("function")
                    .and_then(|f| f.get("arguments"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let args = serde_json::from_str::<Value>(args_raw)
                    .unwrap_or_else(|_| Value::String(args_raw.to_string()));

                assistant_tool_calls.push(json!({
                    "id": call.get("id").cloned().unwrap_or(Value::Null),
                    "name": call.get("function").and_then(|f| f.get("name")).cloned().unwrap_or(Value::Null),
                    "arguments": args,
                }));
            }
        }

        if role == "tool" {
            let content_raw = msg.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let content = serde_json::from_str::<Value>(content_raw)
                .unwrap_or_else(|_| Value::String(content_raw.to_string()));
            tool_messages.push(json!({
                "name": msg.get("name").cloned().unwrap_or(Value::Null),
                "tool_call_id": msg.get("tool_call_id").cloned().unwrap_or(Value::Null),
                "content": content,
            }));
        }
    }

    json!({
        "model": req.get("model").cloned().unwrap_or(Value::Null),
        "temperature": req.get("temperature").and_then(|v| v.as_f64()),
        "tools": tools,
        "message_roles": message_roles,
        "assistant_tool_calls": assistant_tool_calls,
        "tool_messages": tool_messages,
    })
}

#[tokio::test]
#[serial]
async fn replay_fixture_drives_session_and_tool_calls() {
    let fixture = support::openai_compat_fixture::load_json_array(include_str!(
        "fixtures/replay/session_write_file.json"
    ));
    let server = support::openai_compat_fixture::FixtureServer::spawn(fixture).await;

    let tmp = tempfile::tempdir().unwrap();
    let (agent, _paths, workspace_root) = fixture_agent(
        &tmp,
        server.base_url.clone(),
        rexos::security::SecurityConfig::default(),
    );

    let session_id = "s-replay";
    agent
        .set_session_allowed_tools(session_id, vec!["fs_write".to_string()])
        .unwrap();

    let out = agent
        .run_session(
            workspace_root.clone(),
            session_id,
            None,
            "write hello.txt",
            TaskKind::Coding,
        )
        .await
        .unwrap();

    assert_eq!(out, "done");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("hello.txt")).unwrap(),
        "hello"
    );

    let requests = server.requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 2, "expected two chat completions calls");
    assert_eq!(
        compact_request(&requests[0]),
        json!({
            "model": "fixture-model",
            "temperature": 0.0,
            "tools": [{
                "name": "fs_write",
                "type": "function",
                "param_type": "object",
                "required": ["content", "path"],
                "properties": ["content", "path"],
                "additional_properties": false,
            }],
            "message_roles": ["user"],
            "assistant_tool_calls": [],
            "tool_messages": [],
        })
    );
    assert_eq!(
        compact_request(&requests[1]),
        json!({
            "model": "fixture-model",
            "temperature": 0.0,
            "tools": [{
                "name": "fs_write",
                "type": "function",
                "param_type": "object",
                "required": ["content", "path"],
                "properties": ["content", "path"],
                "additional_properties": false,
            }],
            "message_roles": ["user", "assistant", "tool"],
            "assistant_tool_calls": [{
                "id": "call_1",
                "name": "fs_write",
                "arguments": { "path": "hello.txt", "content": "hello" },
            }],
            "tool_messages": [{
                "name": "fs_write",
                "tool_call_id": "call_1",
                "content": "ok",
            }],
        })
    );

    server.abort();
}

#[tokio::test]
#[serial]
async fn replay_fixture_blocks_tool_not_in_allowed_tools() {
    let fixture = support::openai_compat_fixture::load_json_array(include_str!(
        "fixtures/replay/session_tool_not_allowed.json"
    ));
    let server = support::openai_compat_fixture::FixtureServer::spawn(fixture).await;

    let tmp = tempfile::tempdir().unwrap();
    let (agent, _paths, workspace_root) = fixture_agent(
        &tmp,
        server.base_url.clone(),
        rexos::security::SecurityConfig::default(),
    );

    let session_id = "s-replay-deny";
    agent
        .set_session_allowed_tools(session_id, vec!["fs_read".to_string()])
        .unwrap();

    let err = agent
        .run_session(
            workspace_root,
            session_id,
            None,
            "try write",
            TaskKind::Coding,
        )
        .await
        .unwrap_err();
    let err_text = err.to_string();
    assert!(
        err_text.contains("tool not allowed"),
        "expected deny error, got: {err_text}"
    );

    let requests = server.requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 1, "expected one chat completions call");
    assert_eq!(
        compact_request(&requests[0]),
        json!({
            "model": "fixture-model",
            "temperature": 0.0,
            "tools": [{
                "name": "fs_read",
                "type": "function",
                "param_type": "object",
                "required": ["path"],
                "properties": ["path"],
                "additional_properties": false,
            }],
            "message_roles": ["user"],
            "assistant_tool_calls": [],
            "tool_messages": [],
        })
    );

    server.abort();
}

#[tokio::test]
#[serial]
async fn replay_fixture_surfaces_tool_failure_errors() {
    let fixture = support::openai_compat_fixture::load_json_array(include_str!(
        "fixtures/replay/session_tool_failed_invalid_path.json"
    ));
    let server = support::openai_compat_fixture::FixtureServer::spawn(fixture).await;

    let tmp = tempfile::tempdir().unwrap();
    let (agent, _paths, workspace_root) = fixture_agent(
        &tmp,
        server.base_url.clone(),
        rexos::security::SecurityConfig::default(),
    );

    let session_id = "s-replay-tool-failed";
    agent
        .set_session_allowed_tools(session_id, vec!["fs_write".to_string()])
        .unwrap();

    let err = agent
        .run_session(
            workspace_root,
            session_id,
            None,
            "write outside workspace",
            TaskKind::Coding,
        )
        .await
        .unwrap_err();
    let err_text = err.to_string();
    assert!(
        err_text.contains("parent traversal"),
        "expected relative-path validation error, got: {err_text}"
    );
    assert!(
        err_text.contains("fs_write"),
        "expected tool name in error, got: {err_text}"
    );

    let requests = server.requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 1, "expected one chat completions call");
    assert_eq!(
        compact_request(&requests[0]),
        json!({
            "model": "fixture-model",
            "temperature": 0.0,
            "tools": [{
                "name": "fs_write",
                "type": "function",
                "param_type": "object",
                "required": ["content", "path"],
                "properties": ["content", "path"],
                "additional_properties": false,
            }],
            "message_roles": ["user"],
            "assistant_tool_calls": [],
            "tool_messages": [],
        })
    );

    server.abort();
}

#[tokio::test]
#[serial]
async fn replay_fixture_executes_mcp_tool_calls() {
    let fixture = support::openai_compat_fixture::load_json_array(include_str!(
        "fixtures/replay/session_mcp_echo.json"
    ));
    let server = support::openai_compat_fixture::FixtureServer::spawn(fixture).await;

    let tmp = tempfile::tempdir().unwrap();
    let (agent, _paths, workspace_root) = fixture_agent(
        &tmp,
        server.base_url.clone(),
        rexos::security::SecurityConfig::default(),
    );

    let mcp_stub = support::mcp_stub::write_mcp_stub(&workspace_root);

    let session_id = "s-replay-mcp";
    agent
        .set_session_mcp_config(session_id, support::mcp_stub::mcp_config_json(&mcp_stub))
        .unwrap();
    agent
        .set_session_allowed_tools(session_id, vec!["mcp_stub__echo".to_string()])
        .unwrap();

    let out = agent
        .run_session(
            workspace_root,
            session_id,
            None,
            "call mcp",
            TaskKind::Coding,
        )
        .await
        .unwrap();
    assert_eq!(out, "done");

    let requests = server.requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 2, "expected two chat completions calls");
    assert_eq!(
        compact_request(&requests[0]),
        json!({
            "model": "fixture-model",
            "temperature": 0.0,
            "tools": [{
                "name": "mcp_stub__echo",
                "type": "function",
                "param_type": "object",
                "required": ["text"],
                "properties": ["text"],
                "additional_properties": false,
            }],
            "message_roles": ["user"],
            "assistant_tool_calls": [],
            "tool_messages": [],
        })
    );
    assert_eq!(
        compact_request(&requests[1]),
        json!({
            "model": "fixture-model",
            "temperature": 0.0,
            "tools": [{
                "name": "mcp_stub__echo",
                "type": "function",
                "param_type": "object",
                "required": ["text"],
                "properties": ["text"],
                "additional_properties": false,
            }],
            "message_roles": ["user", "assistant", "tool"],
            "assistant_tool_calls": [{
                "id": "call_1",
                "name": "mcp_stub__echo",
                "arguments": { "text": "yo" },
            }],
            "tool_messages": [{
                "name": "mcp_stub__echo",
                "tool_call_id": "call_1",
                "content": { "content": [{ "type": "text", "text": "yo" }] },
            }],
        })
    );

    server.abort();
}

#[tokio::test]
#[serial]
async fn replay_fixture_enforces_tool_approval_for_dangerous_tools() {
    let _mode = EnvVarGuard::set("LOOPFORGE_APPROVAL_MODE", "enforce");
    let _allow = EnvVarGuard::set("LOOPFORGE_APPROVAL_ALLOW", "");

    let fixture = support::openai_compat_fixture::load_json_array(include_str!(
        "fixtures/replay/session_tool_approval_required.json"
    ));
    let server = support::openai_compat_fixture::FixtureServer::spawn(fixture).await;

    let tmp = tempfile::tempdir().unwrap();
    let (agent, _paths, workspace_root) = fixture_agent(
        &tmp,
        server.base_url.clone(),
        rexos::security::SecurityConfig::default(),
    );

    let session_id = "s-replay-approval";
    agent
        .set_session_allowed_tools(session_id, vec!["shell".to_string()])
        .unwrap();

    let err = agent
        .run_session(
            workspace_root,
            session_id,
            None,
            "run shell",
            TaskKind::Coding,
        )
        .await
        .unwrap_err();
    let err_text = err.to_string();
    assert!(
        err_text.contains("approval required for dangerous tool `shell`"),
        "expected approval error, got: {err_text}"
    );

    let requests = server.requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 1, "expected one chat completions call");
    assert_eq!(
        compact_request(&requests[0]),
        json!({
            "model": "fixture-model",
            "temperature": 0.0,
            "tools": [{
                "name": "shell",
                "type": "function",
                "param_type": "object",
                "required": ["command"],
                "properties": ["command", "timeout_ms"],
                "additional_properties": false,
            }],
            "message_roles": ["user"],
            "assistant_tool_calls": [],
            "tool_messages": [],
        })
    );

    server.abort();
}

#[tokio::test]
#[serial]
async fn replay_fixture_blocks_leak_guard_in_enforce_mode() {
    let fixture = support::openai_compat_fixture::load_json_array(include_str!(
        "fixtures/replay/session_leak_guard_enforce.json"
    ));
    let server = support::openai_compat_fixture::FixtureServer::spawn(fixture).await;

    let tmp = tempfile::tempdir().unwrap();
    let mut security = rexos::security::SecurityConfig::default();
    security.leaks.mode = LeakMode::Enforce;

    let (agent, _paths, workspace_root) = fixture_agent(&tmp, server.base_url.clone(), security);
    std::fs::write(
        workspace_root.join("secret.txt"),
        "secret=sk-01234567890123456789",
    )
    .unwrap();

    let session_id = "s-replay-leak-guard";
    agent
        .set_session_allowed_tools(session_id, vec!["fs_read".to_string()])
        .unwrap();

    let err = agent
        .run_session(
            workspace_root,
            session_id,
            None,
            "read secret",
            TaskKind::Coding,
        )
        .await
        .unwrap_err();
    let err_text = err.to_string();
    assert!(
        err_text.contains("tool output blocked by leak guard"),
        "expected leak guard error, got: {err_text}"
    );

    let requests = server.requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 1, "expected one chat completions call");
    assert_eq!(
        compact_request(&requests[0]),
        json!({
            "model": "fixture-model",
            "temperature": 0.0,
            "tools": [{
                "name": "fs_read",
                "type": "function",
                "param_type": "object",
                "required": ["path"],
                "properties": ["path"],
                "additional_properties": false,
            }],
            "message_roles": ["user"],
            "assistant_tool_calls": [],
            "tool_messages": [],
        })
    );

    server.abort();
}

#[tokio::test]
#[serial]
async fn replay_fixture_blocks_egress_policy_rule_mismatches() {
    let fixture = support::openai_compat_fixture::load_json_array(include_str!(
        "fixtures/replay/session_egress_policy_block.json"
    ));
    let server = support::openai_compat_fixture::FixtureServer::spawn(fixture).await;

    let tmp = tempfile::tempdir().unwrap();
    let security = rexos::security::SecurityConfig {
        egress: EgressConfig {
            rules: vec![EgressRule {
                tool: "web_fetch".to_string(),
                host: "example.com".to_string(),
                path_prefix: "/".to_string(),
                methods: vec!["GET".to_string()],
            }],
        },
        ..Default::default()
    };

    let (agent, _paths, workspace_root) = fixture_agent(&tmp, server.base_url.clone(), security);

    let session_id = "s-replay-egress";
    agent
        .set_session_allowed_tools(session_id, vec!["web_fetch".to_string()])
        .unwrap();

    let err = agent
        .run_session(
            workspace_root,
            session_id,
            None,
            "fetch localhost",
            TaskKind::Coding,
        )
        .await
        .unwrap_err();
    let err_text = err.to_string();
    assert!(
        err_text.contains("egress host not allowed"),
        "expected egress host block, got: {err_text}"
    );
    assert!(
        err_text.contains("web_fetch"),
        "expected tool name in error, got: {err_text}"
    );

    let requests = server.requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 1, "expected one chat completions call");
    assert_eq!(
        compact_request(&requests[0]),
        json!({
            "model": "fixture-model",
            "temperature": 0.0,
            "tools": [{
                "name": "web_fetch",
                "type": "function",
                "param_type": "object",
                "required": ["url"],
                "properties": ["allow_private", "max_bytes", "timeout_ms", "url"],
                "additional_properties": false,
            }],
            "message_roles": ["user"],
            "assistant_tool_calls": [],
            "tool_messages": [],
        })
    );

    server.abort();
}

#[tokio::test]
#[serial]
async fn replay_fixture_blocks_egress_policy_path_prefix_mismatches() {
    let fixture = support::openai_compat_fixture::load_json_array(include_str!(
        "fixtures/replay/session_egress_policy_path_block.json"
    ));
    let server = support::openai_compat_fixture::FixtureServer::spawn(fixture).await;

    let tmp = tempfile::tempdir().unwrap();
    let security = rexos::security::SecurityConfig {
        egress: EgressConfig {
            rules: vec![EgressRule {
                tool: "web_fetch".to_string(),
                host: "127.0.0.1".to_string(),
                path_prefix: "/ok".to_string(),
                methods: vec!["GET".to_string()],
            }],
        },
        ..Default::default()
    };

    let (agent, _paths, workspace_root) = fixture_agent(&tmp, server.base_url.clone(), security);

    let session_id = "s-replay-egress-path";
    agent
        .set_session_allowed_tools(session_id, vec!["web_fetch".to_string()])
        .unwrap();

    let err = agent
        .run_session(
            workspace_root,
            session_id,
            None,
            "fetch disallowed path",
            TaskKind::Coding,
        )
        .await
        .unwrap_err();
    let err_text = err.to_string();
    assert!(
        err_text.contains("egress path not allowed"),
        "expected egress path block, got: {err_text}"
    );
    assert!(
        err_text.contains("web_fetch"),
        "expected tool name in error, got: {err_text}"
    );

    let requests = server.requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 1, "expected one chat completions call");
    assert_eq!(
        compact_request(&requests[0]),
        json!({
            "model": "fixture-model",
            "temperature": 0.0,
            "tools": [{
                "name": "web_fetch",
                "type": "function",
                "param_type": "object",
                "required": ["url"],
                "properties": ["allow_private", "max_bytes", "timeout_ms", "url"],
                "additional_properties": false,
            }],
            "message_roles": ["user"],
            "assistant_tool_calls": [],
            "tool_messages": [],
        })
    );

    server.abort();
}

#[tokio::test]
#[serial]
async fn replay_fixture_blocks_egress_policy_method_mismatches() {
    let fixture = support::openai_compat_fixture::load_json_array(include_str!(
        "fixtures/replay/session_egress_policy_method_block.json"
    ));
    let server = support::openai_compat_fixture::FixtureServer::spawn(fixture).await;

    let tmp = tempfile::tempdir().unwrap();
    let security = rexos::security::SecurityConfig {
        egress: EgressConfig {
            rules: vec![EgressRule {
                tool: "a2a_send".to_string(),
                host: "127.0.0.1".to_string(),
                path_prefix: "/".to_string(),
                methods: vec!["GET".to_string()],
            }],
        },
        ..Default::default()
    };

    let (agent, _paths, workspace_root) = fixture_agent(&tmp, server.base_url.clone(), security);

    let session_id = "s-replay-egress-method";
    agent
        .set_session_allowed_tools(session_id, vec!["a2a_send".to_string()])
        .unwrap();

    let err = agent
        .run_session(
            workspace_root,
            session_id,
            None,
            "send a2a disallowed method",
            TaskKind::Coding,
        )
        .await
        .unwrap_err();
    let err_text = err.to_string();
    assert!(
        err_text.contains("egress method not allowed"),
        "expected egress method block, got: {err_text}"
    );
    assert!(
        err_text.contains("a2a_send"),
        "expected tool name in error, got: {err_text}"
    );

    let requests = server.requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 1, "expected one chat completions call");
    assert_eq!(
        compact_request(&requests[0]),
        json!({
            "model": "fixture-model",
            "temperature": 0.0,
            "tools": [{
                "name": "a2a_send",
                "type": "function",
                "param_type": "object",
                "required": ["message"],
                "properties": ["agent_url", "allow_private", "message", "session_id", "url"],
                "additional_properties": false,
            }],
            "message_roles": ["user"],
            "assistant_tool_calls": [],
            "tool_messages": [],
        })
    );

    server.abort();
}

#[tokio::test]
#[serial]
async fn replay_fixture_blocks_a2a_discover_egress_path_mismatches() {
    let fixture = support::openai_compat_fixture::load_json_array(include_str!(
        "fixtures/replay/session_egress_policy_a2a_discover_path_block.json"
    ));
    let server = support::openai_compat_fixture::FixtureServer::spawn(fixture).await;

    let tmp = tempfile::tempdir().unwrap();
    let security = rexos::security::SecurityConfig {
        egress: EgressConfig {
            rules: vec![EgressRule {
                tool: "a2a_discover".to_string(),
                host: "127.0.0.1".to_string(),
                path_prefix: "/ok".to_string(),
                methods: vec!["GET".to_string()],
            }],
        },
        ..Default::default()
    };

    let (agent, _paths, workspace_root) = fixture_agent(&tmp, server.base_url.clone(), security);

    let session_id = "s-replay-egress-a2a-discover";
    agent
        .set_session_allowed_tools(session_id, vec!["a2a_discover".to_string()])
        .unwrap();

    let err = agent
        .run_session(
            workspace_root,
            session_id,
            None,
            "discover agent card",
            TaskKind::Coding,
        )
        .await
        .unwrap_err();
    let err_text = err.to_string();
    assert!(
        err_text.contains("egress path not allowed"),
        "expected egress path block, got: {err_text}"
    );
    assert!(
        err_text.contains("/.well-known/agent.json"),
        "expected agent card path in error, got: {err_text}"
    );
    assert!(
        err_text.contains("a2a_discover"),
        "expected tool name in error, got: {err_text}"
    );

    let requests = server.requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 1, "expected one chat completions call");
    assert_eq!(
        compact_request(&requests[0]),
        json!({
            "model": "fixture-model",
            "temperature": 0.0,
            "tools": [{
                "name": "a2a_discover",
                "type": "function",
                "param_type": "object",
                "required": ["url"],
                "properties": ["allow_private", "url"],
                "additional_properties": false,
            }],
            "message_roles": ["user"],
            "assistant_tool_calls": [],
            "tool_messages": [],
        })
    );

    server.abort();
}
