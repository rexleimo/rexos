use super::super::mcp_sanitize::sanitize_mcp_config;
use super::diagnose::collect_mcp_diagnose_data;
use super::json::{build_mcp_diagnose_json, normalize_json_string};
use std::path::Path;

const MCP_STUB_PY: &str = r#"
import json
import sys

def send(obj):
    sys.stdout.write(json.dumps(obj) + "\n")
    sys.stdout.flush()

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        msg = json.loads(line)
    except Exception:
        continue

    method = msg.get("method")
    if not method:
        continue

    # Notifications have no id; ignore them.
    if "id" not in msg:
        continue

    msg_id = msg.get("id")
    params = msg.get("params") or {}

    if method == "initialize":
        send({"jsonrpc": "2.0", "id": msg_id, "result": {}})
    elif method == "tools/list":
        send({
            "jsonrpc": "2.0",
            "id": msg_id,
            "result": {
                "tools": [
                    {
                        "name": "echo",
                        "description": "Echo input text",
                        "inputSchema": {
                            "type": "object",
                            "properties": {"text": {"type": "string"}},
                            "required": ["text"],
                            "additionalProperties": False
                        }
                    }
                ]
            }
        })
    elif method == "resources/list":
        send({
            "jsonrpc": "2.0",
            "id": msg_id,
            "result": {
                "resources": [{"uri": "mem://hello", "name": "hello", "mimeType": "text/plain"}],
                "nextCursor": None,
            }
        })
    elif method == "prompts/list":
        send({
            "jsonrpc": "2.0",
            "id": msg_id,
            "result": {
                "prompts": [{"name": "greet", "description": "greet prompt"}],
                "nextCursor": None,
            }
        })
    else:
        send({"jsonrpc": "2.0", "id": msg_id, "error": {"code": -32601, "message": "unknown method"}})
"#;

fn python_exe() -> &'static str {
    if cfg!(windows) {
        "python"
    } else {
        "python3"
    }
}

fn write_mcp_stub(root: &Path) -> std::path::PathBuf {
    let path = root.join("mcp_diagnose_stub.py");
    std::fs::write(&path, MCP_STUB_PY).expect("write mcp diagnose stub script");
    path
}

fn mcp_config_json(script: &Path) -> String {
    serde_json::json!({
        "servers": {
            "stub": {
                "command": python_exe(),
                "args": ["-u", script.to_string_lossy()],
                "cwd": ".",
                "env": {
                    "API_KEY": "secret",
                }
            }
        }
    })
    .to_string()
}

#[test]
fn build_mcp_diagnose_json_includes_expected_keys_and_redaction() {
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("workspace");

    let config = serde_json::json!({
        "servers": {
            "s1": {
                "command": "python",
                "env": { "API_KEY": "secret" },
            }
        }
    });
    let sanitized = sanitize_mcp_config(&config);

    let out = build_mcp_diagnose_json(
        &workspace,
        "s-test",
        sanitized,
        serde_json::json!(["s1"]),
        vec!["mcp_s1__echo".to_string()],
        None,
        None,
    )
    .unwrap();

    assert_eq!(
        out["workspace"].as_str(),
        Some(workspace.display().to_string().as_str())
    );
    assert_eq!(out["session_id"].as_str(), Some("s-test"));
    assert!(out.get("servers").is_some());
    assert!(out.get("tool_names").is_some());
    assert_eq!(
        out["config"]["servers"]["s1"]["env"]["API_KEY"].as_str(),
        Some("[redacted]")
    );

    let tool_names: Vec<String> = out["tool_names"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();
    assert_eq!(tool_names, vec!["mcp_s1__echo".to_string()]);
    assert!(out.get("resources").is_none());
    assert!(out.get("prompts").is_none());
}

#[test]
fn normalize_json_string_round_trips() {
    let out = normalize_json_string(" {\"servers\":{}} ").unwrap();
    assert_eq!(out, "{\"servers\":{}}");
}

#[test]
fn sanitize_mcp_config_redacts_env_values() {
    let input = serde_json::json!({
        "servers": {
            "s1": {
                "env": { "API_KEY": "secret" },
                "command": "python"
            }
        }
    });
    let sanitized = sanitize_mcp_config(&input);
    assert_eq!(
        sanitized["servers"]["s1"]["env"]["API_KEY"].as_str(),
        Some("[redacted]")
    );
    assert_eq!(
        sanitized["servers"]["s1"]["command"].as_str(),
        Some("python")
    );
}

#[tokio::test]
async fn collect_mcp_diagnose_data_includes_tools_resources_prompts_and_redaction() {
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let stub = write_mcp_stub(tmp.path());
    let config_json = mcp_config_json(&stub);

    let out = collect_mcp_diagnose_data(
        workspace.clone(),
        &config_json,
        rexos::security::SecurityConfig::default(),
        true,
        true,
    )
    .await
    .unwrap();

    assert_eq!(out.servers_json, serde_json::json!(["stub"]));
    assert!(out.tool_names.contains(&"mcp_stub__echo".to_string()));
    assert_eq!(
        out.sanitized_config["servers"]["stub"]["env"]["API_KEY"].as_str(),
        Some("[redacted]")
    );
    let resources = out
        .resources_json
        .as_ref()
        .expect("resources should be present");
    assert_eq!(resources[0]["server"].as_str(), Some("stub"));
    assert_eq!(
        resources[0]["resources"][0]["uri"].as_str(),
        Some("mem://hello")
    );
    let prompts = out
        .prompts_json
        .as_ref()
        .expect("prompts should be present");
    assert_eq!(prompts[0]["server"].as_str(), Some("stub"));
    assert_eq!(prompts[0]["prompts"][0]["name"].as_str(), Some("greet"));
}

#[tokio::test]
async fn collect_mcp_diagnose_data_skips_optional_lists_when_disabled() {
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let stub = write_mcp_stub(tmp.path());
    let config_json = mcp_config_json(&stub);

    let out = collect_mcp_diagnose_data(
        workspace,
        &config_json,
        rexos::security::SecurityConfig::default(),
        false,
        false,
    )
    .await
    .unwrap();

    assert!(out.resources_json.is_none());
    assert!(out.prompts_json.is_none());
}
