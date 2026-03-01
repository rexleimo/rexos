use axum::routing::get;
use axum::Router;

#[tokio::test]
async fn web_fetch_rejects_non_http_schemes() {
    let tmp = tempfile::tempdir().unwrap();
    let tools = rexos::tools::Toolset::new(tmp.path().to_path_buf()).unwrap();

    let err = tools
        .call(
            "web_fetch",
            r#"{ "url": "file:///etc/passwd", "allow_private": false }"#,
        )
        .await
        .unwrap_err();
    assert!(err.to_string().contains("http"), "{err}");
}

#[tokio::test]
async fn web_fetch_denies_loopback_by_default() {
    let tmp = tempfile::tempdir().unwrap();
    let tools = rexos::tools::Toolset::new(tmp.path().to_path_buf()).unwrap();

    let err = tools
        .call(
            "web_fetch",
            r#"{ "url": "http://127.0.0.1:1/", "allow_private": false }"#,
        )
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("loopback") || msg.contains("private") || msg.contains("denied"),
        "{msg}"
    );
}

#[tokio::test]
async fn web_fetch_allows_loopback_when_allow_private_true() {
    async fn handler() -> &'static str {
        "hello"
    }

    let app = Router::new().route("/", get(handler));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let tmp = tempfile::tempdir().unwrap();
    let tools = rexos::tools::Toolset::new(tmp.path().to_path_buf()).unwrap();

    let out = tools
        .call(
            "web_fetch",
            &format!(
                r#"{{ "url": "http://{addr}/", "allow_private": true, "max_bytes": 1000 }}"#
            ),
        )
        .await
        .unwrap();

    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["status"], 200);
    assert_eq!(v["body"], "hello");

    server.abort();
}

