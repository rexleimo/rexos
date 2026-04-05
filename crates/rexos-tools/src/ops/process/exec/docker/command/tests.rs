#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;

use super::output::docker_exec_result;

#[test]
fn docker_exec_result_includes_image_and_workdir() {
    let output = std::process::Output {
        status: std::process::ExitStatus::from_raw(0),
        stdout: b"hello".to_vec(),
        stderr: Vec::new(),
    };

    let result = docker_exec_result(output, "alpine:3".to_string());
    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["image"], "alpine:3");
    assert_eq!(value["workdir"], "/workspace");
    assert_eq!(value["stdout"], "hello");
}
