use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;

use super::super::discovery::{find_or_create_page_ws, read_devtools_url};
use super::shared::{async_env_lock, EnvVarGuard};

#[tokio::test]
async fn find_or_create_page_ws_bypasses_proxy_for_loopback() {
    async fn new_handler() -> Json<serde_json::Value> {
        Json(json!({
            "webSocketDebuggerUrl": "ws://127.0.0.1/devtools/page/1"
        }))
    }

    async fn list_handler() -> Json<serde_json::Value> {
        Json(json!([{
            "type": "page",
            "webSocketDebuggerUrl": "ws://127.0.0.1/devtools/page/1"
        }]))
    }

    let app = Router::new()
        .route("/json/new", get(new_handler))
        .route("/json/list", get(list_handler));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let proxy = reqwest::Proxy::http("http://127.0.0.1:1").unwrap();
    let http = reqwest::Client::builder()
        .proxy(proxy)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    let base = reqwest::Url::parse(&format!("http://{addr}")).unwrap();
    let ws = find_or_create_page_ws(&http, &base).await.unwrap();
    assert_eq!(ws, "ws://127.0.0.1/devtools/page/1");

    server.abort();
}

#[tokio::test]
async fn cdp_tab_mode_reuse_skips_json_new() {
    #[derive(Clone)]
    struct StateData {
        calls_new: Arc<AtomicUsize>,
    }

    async fn new_handler(State(state): State<StateData>) -> (StatusCode, Json<serde_json::Value>) {
        state.calls_new.fetch_add(1, AtomicOrdering::Relaxed);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "disabled" })),
        )
    }

    async fn list_handler() -> Json<serde_json::Value> {
        Json(json!([{
            "type": "page",
            "webSocketDebuggerUrl": "ws://127.0.0.1/devtools/page/reuse"
        }]))
    }

    let state = StateData {
        calls_new: Arc::new(AtomicUsize::new(0)),
    };
    let app = Router::new()
        .route("/json/new", get(new_handler))
        .route("/json/list", get(list_handler))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let _lock = async_env_lock().lock().await;
    let _mode = EnvVarGuard::set("LOOPFORGE_BROWSER_CDP_TAB_MODE", "reuse");

    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let base = reqwest::Url::parse(&format!("http://{addr}")).unwrap();

    let ws = find_or_create_page_ws(&http, &base).await.unwrap();
    assert_eq!(ws, "ws://127.0.0.1/devtools/page/reuse");
    assert_eq!(state.calls_new.load(AtomicOrdering::Relaxed), 0);

    server.abort();
}

#[tokio::test]
async fn read_devtools_url_includes_stderr_tail_on_exit() {
    let mut command = if cfg!(windows) {
        let mut command = tokio::process::Command::new("powershell");
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            "[Console]::Error.WriteLine('ERR_LINE_1'); [Console]::Error.WriteLine('ERR_LINE_2'); exit 1",
        ]);
        command
    } else {
        let mut command = tokio::process::Command::new("bash");
        command.args(["-lc", "echo ERR_LINE_1 1>&2; echo ERR_LINE_2 1>&2; exit 1"]);
        command
    };

    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());

    let mut child = command.spawn().unwrap();
    let stderr = child.stderr.take().unwrap();

    let err = read_devtools_url(stderr).await.unwrap_err();
    let message = err.to_string();
    assert!(message.contains("ERR_LINE_1"), "{message}");
    assert!(message.contains("ERR_LINE_2"), "{message}");

    let _ = child.wait().await;
}
