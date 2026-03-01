use std::fs;

#[tokio::test]
async fn fs_tools_respect_workspace_root() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::write(root.join("a.txt"), "hello").unwrap();

    let tools = rexos::tools::Toolset::new(root.to_path_buf()).unwrap();

    let content = tools
        .call("fs_read", r#"{ "path": "a.txt" }"#)
        .await
        .unwrap();
    assert_eq!(content, "hello");

    tools
        .call("fs_write", r#"{ "path": "b/b.txt", "content": "world" }"#)
        .await
        .unwrap();
    assert_eq!(fs::read_to_string(root.join("b/b.txt")).unwrap(), "world");

    let err = tools
        .call("fs_read", r#"{ "path": "../secret.txt" }"#)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("parent traversal"));
}

#[tokio::test]
async fn shell_tool_runs_in_workspace() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    let tools = rexos::tools::Toolset::new(root.to_path_buf()).unwrap();
    let out = tools
        .call("shell", r#"{ "command": "pwd" }"#)
        .await
        .unwrap();

    let expected = root.canonicalize().unwrap();
    let got_raw = out.trim().to_string();
    let got_path = std::path::PathBuf::from(&got_raw);
    let got = got_path
        .canonicalize()
        .unwrap_or_else(|e| panic!("canonicalize pwd output failed: {e}; output={got_raw:?}"));
    assert_eq!(got, expected);
}
