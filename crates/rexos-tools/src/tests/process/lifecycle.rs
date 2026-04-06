use super::*;

#[tokio::test]
async fn docker_exec_is_disabled_by_default() {
    let _lock = async_env_lock().lock().await;

    let previous = std::env::var_os("LOOPFORGE_DOCKER_EXEC_ENABLED");
    std::env::remove_var("LOOPFORGE_DOCKER_EXEC_ENABLED");

    let tmp = tempfile::tempdir().unwrap();
    let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();
    let err = tools
        .call("docker_exec", r#"{ "command": "echo hi" }"#)
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("LOOPFORGE_DOCKER_EXEC_ENABLED") || msg.contains("disabled"),
        "{msg}"
    );

    match previous {
        Some(v) => std::env::set_var("LOOPFORGE_DOCKER_EXEC_ENABLED", v),
        None => std::env::remove_var("LOOPFORGE_DOCKER_EXEC_ENABLED"),
    }
}

#[tokio::test]
async fn process_tools_start_poll_write_kill_and_list() {
    use std::time::Duration;

    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("ws");
    std::fs::create_dir_all(&workspace).unwrap();

    let tools = Toolset::new(workspace).unwrap();

    let start_args = if cfg!(windows) {
        serde_json::json!({
            "command": "powershell",
            "args": [
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "[Console]::Out.WriteLine('READY'); [Console]::Out.Flush(); $line = [Console]::In.ReadLine(); [Console]::Out.WriteLine(('ECHO:' + $line)); [Console]::Out.Flush(); Start-Sleep -Seconds 5"
            ]
        })
    } else {
        serde_json::json!({
            "command": "bash",
            "args": ["-lc", "echo READY; read line; echo ECHO:$line; sleep 5"]
        })
    };

    let out = tools
        .call("process_start", &start_args.to_string())
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).expect("process_start is json");
    let process_id = v
        .get("process_id")
        .and_then(|v| v.as_str())
        .expect("process_id")
        .to_string();

    let list = tools.call("process_list", r#"{}"#).await.unwrap();
    let lv: serde_json::Value = serde_json::from_str(&list).expect("process_list is json");
    let arr = lv.as_array().expect("process_list output is array");
    assert!(
        arr.iter()
            .any(|p| { p.get("process_id").and_then(|v| v.as_str()) == Some(process_id.as_str()) }),
        "process_list did not include {process_id}: {lv}"
    );

    let ready_timeout = if cfg!(windows) {
        Duration::from_secs(8)
    } else {
        Duration::from_secs(2)
    };

    let mut seen_out = String::new();
    let mut seen_err = String::new();
    let deadline = tokio::time::Instant::now() + ready_timeout;
    loop {
        let poll = tools
            .call(
                "process_poll",
                &format!(r#"{{ "process_id": "{}" }}"#, process_id),
            )
            .await
            .unwrap();
        let pv: serde_json::Value = serde_json::from_str(&poll).expect("process_poll is json");
        let stdout = pv.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
        let stderr = pv.get("stderr").and_then(|v| v.as_str()).unwrap_or("");
        seen_out.push_str(stdout);
        seen_err.push_str(stderr);
        if seen_out.contains("READY") || seen_err.contains("READY") {
            break;
        }
        if pv.get("alive").and_then(|v| v.as_bool()) == Some(false) {
            panic!(
                "process exited before READY (exit_code={:?})\nstdout:\n{}\nstderr:\n{}",
                pv.get("exit_code"),
                seen_out,
                seen_err
            );
        }
        if tokio::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(
        seen_out.contains("READY") || seen_err.contains("READY"),
        "did not see READY\nstdout:\n{}\nstderr:\n{}",
        seen_out,
        seen_err
    );

    let _ = tools
        .call(
            "process_write",
            &format!(r#"{{ "process_id": "{}", "data": "hi" }}"#, process_id),
        )
        .await
        .unwrap();

    let mut seen_out = String::new();
    let mut seen_err = String::new();
    let deadline = tokio::time::Instant::now() + ready_timeout;
    loop {
        let poll = tools
            .call(
                "process_poll",
                &format!(r#"{{ "process_id": "{}" }}"#, process_id),
            )
            .await
            .unwrap();
        let pv: serde_json::Value = serde_json::from_str(&poll).expect("process_poll is json");
        let stdout = pv.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
        let stderr = pv.get("stderr").and_then(|v| v.as_str()).unwrap_or("");
        seen_out.push_str(stdout);
        seen_err.push_str(stderr);
        if seen_out.contains("ECHO:hi") || seen_err.contains("ECHO:hi") {
            break;
        }
        if pv.get("alive").and_then(|v| v.as_bool()) == Some(false) {
            panic!(
                "process exited before ECHO:hi (exit_code={:?})\nstdout:\n{}\nstderr:\n{}",
                pv.get("exit_code"),
                seen_out,
                seen_err
            );
        }
        if tokio::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(
        seen_out.contains("ECHO:hi") || seen_err.contains("ECHO:hi"),
        "did not see ECHO:hi\nstdout:\n{}\nstderr:\n{}",
        seen_out,
        seen_err
    );

    let _ = tools
        .call(
            "process_kill",
            &format!(r#"{{ "process_id": "{}" }}"#, process_id),
        )
        .await
        .unwrap();

    let list = tools.call("process_list", r#"{}"#).await.unwrap();
    let lv: serde_json::Value = serde_json::from_str(&list).expect("process_list is json");
    let arr = lv.as_array().expect("process_list output is array");
    assert!(
        !arr.iter()
            .any(|p| { p.get("process_id").and_then(|v| v.as_str()) == Some(process_id.as_str()) }),
        "process still listed after kill: {lv}"
    );
}
