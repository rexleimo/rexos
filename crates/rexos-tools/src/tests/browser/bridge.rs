use super::shared::{stub_bridge_script, EnvVarGuard};
use super::*;

#[tokio::test]
async fn browser_tools_work_with_stub_bridge() {
    let _lock = async_env_lock().lock().await;

    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("ws");
    std::fs::create_dir_all(&workspace).unwrap();

    let bridge_path = tmp.path().join("bridge.py");
    std::fs::write(&bridge_path, stub_bridge_script()).unwrap();

    let python = if cfg!(windows) { "python" } else { "python3" };
    let _backend_guard = EnvVarGuard::set("LOOPFORGE_BROWSER_BACKEND", "playwright");
    let _python_guard = EnvVarGuard::set("LOOPFORGE_BROWSER_PYTHON", python);
    let _bridge_guard = EnvVarGuard::set("LOOPFORGE_BROWSER_BRIDGE_PATH", bridge_path.as_os_str());

    let tools = Toolset::new(workspace.clone()).unwrap();

    let _ = tools
        .call(
            "browser_navigate",
            r#"{ "url": "http://127.0.0.1:1/", "allow_private": true }"#,
        )
        .await
        .unwrap();

    let out = tools
        .call("browser_run_js", r#"{ "expression": "1 + 1" }"#)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["result"], 2);

    let out = tools
        .call(
            "browser_scroll",
            r#"{ "direction": "down", "amount": 123 }"#,
        )
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["scrollY"], 123);

    let out = tools
        .call("browser_press_key", r#"{ "key": "Enter" }"#)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["key"], "Enter");

    let out = tools
        .call(
            "browser_wait",
            r##"{ "selector": "#content", "timeout_ms": 1 }"##,
        )
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["waited_for"]["selector"], "#content");

    let out = tools
        .call(
            "browser_wait_for",
            r#"{ "text": "hello", "timeout_ms": 1 }"#,
        )
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["waited_for"]["text"], "hello");

    let page = tools.call("browser_read_page", r#"{}"#).await.unwrap();
    let v: serde_json::Value = serde_json::from_str(&page).unwrap();
    assert_eq!(v["title"], "Stub");
    assert_eq!(v["content"], "hello");

    let _ = tools
        .call("browser_screenshot", r#"{ "path": "shot.png" }"#)
        .await
        .unwrap();
    let bytes = std::fs::read(workspace.join("shot.png")).unwrap();
    assert!(
        bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]),
        "not a PNG"
    );

    let out = tools.call("browser_back", r#"{}"#).await.unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v.get("url").and_then(|v| v.as_str()).is_some(), "{v}");

    let out = tools.call("browser_close", r#"{}"#).await.unwrap();
    assert_eq!(out.trim(), "ok");
}

#[tokio::test]
async fn browser_navigate_honors_headless_flag() {
    let _lock = async_env_lock().lock().await;

    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("ws");
    std::fs::create_dir_all(&workspace).unwrap();

    let bridge_path = tmp.path().join("bridge.py");
    std::fs::write(&bridge_path, stub_bridge_script()).unwrap();

    let python = if cfg!(windows) { "python" } else { "python3" };
    let _backend_guard = EnvVarGuard::set("LOOPFORGE_BROWSER_BACKEND", "playwright");
    let _python_guard = EnvVarGuard::set("LOOPFORGE_BROWSER_PYTHON", python);
    let _bridge_guard = EnvVarGuard::set("LOOPFORGE_BROWSER_BRIDGE_PATH", bridge_path.as_os_str());

    let tools = Toolset::new(workspace).unwrap();

    let out = tools
        .call(
            "browser_navigate",
            r#"{ "url": "http://127.0.0.1:1/", "allow_private": true, "headless": false }"#,
        )
        .await
        .unwrap();

    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(
        v.get("headless").and_then(|v| v.as_bool()),
        Some(false),
        "{v}"
    );
}
