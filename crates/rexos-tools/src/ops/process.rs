use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{ProcessEntry, ProcessOutputBuffer, Toolset, PROCESS_MAX_PROCESSES};

impl Toolset {
    pub(crate) async fn shell(
        &self,
        command: &str,
        timeout_ms: Option<u64>,
    ) -> anyhow::Result<String> {
        if command.trim().is_empty() {
            bail!("command is empty");
        }

        // Basic guardrail: avoid obvious foot-guns.
        if command.contains("rm -rf /") || command.contains("sudo ") {
            bail!("command denied by policy");
        }

        let timeout = Duration::from_millis(timeout_ms.unwrap_or(60_000));

        let mut cmd = if cfg!(windows) {
            let mut cmd = tokio::process::Command::new("powershell");
            cmd.args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
            ]);

            let wrapped = format!(
                "$ErrorActionPreference = 'Stop'; $global:LASTEXITCODE = 0; {command}; if ($global:LASTEXITCODE -ne 0) {{ exit $global:LASTEXITCODE }}",
                command = command
            );
            cmd.arg(wrapped);
            cmd
        } else {
            let mut cmd = tokio::process::Command::new("bash");
            cmd.arg("-c").arg(command);
            cmd
        };

        cmd.current_dir(&self.workspace_root).env_clear();

        if let Ok(path) = std::env::var("PATH") {
            cmd.env("PATH", path);
        }

        if cfg!(windows) {
            for key in ["SystemRoot", "USERPROFILE", "TEMP", "TMP"] {
                if let Ok(v) = std::env::var(key) {
                    cmd.env(key, v);
                }
            }
        } else {
            for key in ["HOME", "USER"] {
                if let Ok(v) = std::env::var(key) {
                    cmd.env(key, v);
                }
            }
        }

        for key in ["CARGO_HOME", "RUSTUP_HOME"] {
            if let Ok(v) = std::env::var(key) {
                cmd.env(key, v);
            }
        }

        let output = tokio::time::timeout(timeout, cmd.output())
            .await
            .context("shell timed out")?
            .context("spawn shell")?;

        let mut combined = String::new();
        combined.push_str(&String::from_utf8_lossy(&output.stdout));
        combined.push_str(&String::from_utf8_lossy(&output.stderr));

        if !output.status.success() {
            bail!("shell failed: {}", combined.trim());
        }

        Ok(combined)
    }

    pub(crate) async fn docker_exec(&self, command: &str) -> anyhow::Result<String> {
        let enabled = std::env::var("LOOPFORGE_DOCKER_EXEC_ENABLED")
            .ok()
            .map(|v| v.trim() == "1")
            .unwrap_or(false);
        if !enabled {
            bail!("docker_exec is disabled (set LOOPFORGE_DOCKER_EXEC_ENABLED=1 to enable)");
        }

        if command.trim().is_empty() {
            bail!("command is empty");
        }

        let image = std::env::var("LOOPFORGE_DOCKER_EXEC_IMAGE")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| "alpine:3.20".to_string());

        let timeout = Duration::from_secs(60);
        let mount = format!("{}:/workspace", self.workspace_root.display());

        let mut cmd = tokio::process::Command::new("docker");
        cmd.arg("run")
            .arg("--rm")
            .arg("-i")
            .arg("--network")
            .arg("none")
            .arg("-v")
            .arg(mount)
            .arg("-w")
            .arg("/workspace")
            .arg(&image)
            .arg("sh")
            .arg("-lc")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = tokio::time::timeout(timeout, cmd.output())
            .await
            .context("docker_exec timed out")?
            .context("spawn docker")?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        Ok(serde_json::json!({
            "exit_code": exit_code,
            "stdout": stdout,
            "stderr": stderr,
            "image": image,
            "workdir": "/workspace",
        })
        .to_string())
    }

    pub(crate) fn spawn_process_output_reader(
        mut stream: impl tokio::io::AsyncRead + Unpin + Send + 'static,
        buffer: Arc<tokio::sync::Mutex<ProcessOutputBuffer>>,
    ) {
        tokio::spawn(async move {
            let mut tmp = [0u8; 4096];
            loop {
                let n = match stream.read(&mut tmp).await {
                    Ok(n) => n,
                    Err(_) => break,
                };
                if n == 0 {
                    break;
                }

                let mut buf = buffer.lock().await;
                buf.push(&tmp[..n]);
            }
        });
    }

    pub(crate) fn decode_process_output(bytes: Vec<u8>) -> String {
        if bytes.is_empty() {
            return String::new();
        }

        // PowerShell often emits UTF-16LE when stdout is piped. If we treat that as UTF-8, we end
        // up with NULs between characters and string matching becomes unreliable.
        let nuls = bytes.iter().filter(|&&b| b == 0).count();
        let nul_ratio = nuls as f32 / bytes.len() as f32;
        if bytes.len() >= 4 && nul_ratio >= 0.20 {
            // Detect endianness via which byte positions are mostly NUL.
            let mut even_nuls = 0usize;
            let mut odd_nuls = 0usize;
            for (idx, b) in bytes.iter().enumerate() {
                if *b == 0 {
                    if idx % 2 == 0 {
                        even_nuls += 1;
                    } else {
                        odd_nuls += 1;
                    }
                }
            }

            let pairs = (bytes.len() / 2).max(1) as f32;
            let even_ratio = even_nuls as f32 / pairs;
            let odd_ratio = odd_nuls as f32 / pairs;

            let is_likely_le = odd_ratio > 0.60 && even_ratio < 0.40;
            let is_likely_be = even_ratio > 0.60 && odd_ratio < 0.40;

            if is_likely_le || is_likely_be {
                let mut u16s = Vec::with_capacity(bytes.len() / 2);
                let mut iter = bytes.chunks_exact(2);
                for chunk in &mut iter {
                    let v = if is_likely_be {
                        u16::from_be_bytes([chunk[0], chunk[1]])
                    } else {
                        u16::from_le_bytes([chunk[0], chunk[1]])
                    };
                    u16s.push(v);
                }

                // Drop an initial BOM if present.
                if u16s.first() == Some(&0xFEFF) {
                    u16s.remove(0);
                }

                return String::from_utf16_lossy(&u16s);
            }
        }

        String::from_utf8_lossy(&bytes).to_string()
    }

    pub(crate) async fn process_start(
        &self,
        command: &str,
        args: &[String],
    ) -> anyhow::Result<String> {
        if command.trim().is_empty() {
            bail!("command is empty");
        }

        let mut mgr = self.processes.lock().await;
        if mgr.processes.len() >= PROCESS_MAX_PROCESSES {
            bail!("process limit reached (max {PROCESS_MAX_PROCESSES})");
        }

        let process_id = uuid::Uuid::new_v4().to_string();

        let mut cmd = tokio::process::Command::new(command);
        cmd.args(args)
            .current_dir(&self.workspace_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_clear();

        if let Ok(path) = std::env::var("PATH") {
            cmd.env("PATH", path);
        }

        if cfg!(windows) {
            for key in ["SystemRoot", "USERPROFILE", "TEMP", "TMP"] {
                if let Ok(v) = std::env::var(key) {
                    cmd.env(key, v);
                }
            }
        } else {
            for key in ["HOME", "USER"] {
                if let Ok(v) = std::env::var(key) {
                    cmd.env(key, v);
                }
            }
        }

        for key in ["CARGO_HOME", "RUSTUP_HOME"] {
            if let Ok(v) = std::env::var(key) {
                cmd.env(key, v);
            }
        }

        let mut child = cmd.spawn().context("spawn process")?;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().context("process stdout is not piped")?;
        let stderr = child.stderr.take().context("process stderr is not piped")?;

        let stdout_buf = Arc::new(tokio::sync::Mutex::new(ProcessOutputBuffer::default()));
        let stderr_buf = Arc::new(tokio::sync::Mutex::new(ProcessOutputBuffer::default()));
        Self::spawn_process_output_reader(stdout, stdout_buf.clone());
        Self::spawn_process_output_reader(stderr, stderr_buf.clone());

        let entry = ProcessEntry {
            command: command.to_string(),
            args: args.to_vec(),
            started_at: std::time::Instant::now(),
            exit_code: None,
            child,
            stdin,
            stdout: stdout_buf,
            stderr: stderr_buf,
        };

        mgr.processes
            .insert(process_id.clone(), Arc::new(tokio::sync::Mutex::new(entry)));

        Ok(serde_json::json!({
            "process_id": process_id,
            "status": "started"
        })
        .to_string())
    }

    pub(crate) async fn process_poll(&self, process_id: &str) -> anyhow::Result<String> {
        let entry = {
            let mgr = self.processes.lock().await;
            mgr.processes
                .get(process_id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("unknown process_id: {process_id}"))?
        };

        let (stdout_buf, stderr_buf, exit_code, alive) = {
            let mut guard = entry.lock().await;
            if guard.exit_code.is_none() {
                if let Some(status) = guard.child.try_wait().context("try_wait process")? {
                    guard.exit_code = Some(status.code().unwrap_or(-1));
                }
            }

            (
                guard.stdout.clone(),
                guard.stderr.clone(),
                guard.exit_code,
                guard.exit_code.is_none(),
            )
        };

        let (stdout, stdout_truncated) = {
            let mut buf = stdout_buf.lock().await;
            buf.take_text()
        };
        let (stderr, stderr_truncated) = {
            let mut buf = stderr_buf.lock().await;
            buf.take_text()
        };

        Ok(serde_json::json!({
            "stdout": stdout,
            "stderr": stderr,
            "stdout_truncated": stdout_truncated,
            "stderr_truncated": stderr_truncated,
            "exit_code": exit_code,
            "alive": alive,
        })
        .to_string())
    }

    pub(crate) async fn process_write(
        &self,
        process_id: &str,
        data: &str,
    ) -> anyhow::Result<String> {
        let entry = {
            let mgr = self.processes.lock().await;
            mgr.processes
                .get(process_id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("unknown process_id: {process_id}"))?
        };

        let data = if data.ends_with('\n') {
            data.to_string()
        } else {
            format!("{data}\n")
        };

        let timeout = Duration::from_secs(5);
        let mut guard = entry.lock().await;
        let stdin = guard.stdin.as_mut().context("process stdin is closed")?;

        tokio::time::timeout(timeout, stdin.write_all(data.as_bytes()))
            .await
            .context("process_write timed out")?
            .context("write stdin")?;
        tokio::time::timeout(timeout, stdin.flush())
            .await
            .context("process_write timed out")?
            .context("flush stdin")?;

        Ok(r#"{"status":"written"}"#.to_string())
    }

    pub(crate) async fn process_kill(&self, process_id: &str) -> anyhow::Result<String> {
        let entry = {
            let mut mgr = self.processes.lock().await;
            mgr.processes
                .remove(process_id)
                .ok_or_else(|| anyhow::anyhow!("unknown process_id: {process_id}"))?
        };

        let mut guard = entry.lock().await;
        let _ = guard.child.kill().await;
        let _ = guard.child.wait().await;

        Ok(r#"{"status":"killed"}"#.to_string())
    }

    pub(crate) async fn process_list(&self) -> anyhow::Result<String> {
        let entries: Vec<(String, Arc<tokio::sync::Mutex<ProcessEntry>>)> = {
            let mgr = self.processes.lock().await;
            mgr.processes
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        };

        let mut out = Vec::new();
        for (id, entry) in entries {
            let mut guard = entry.lock().await;
            if guard.exit_code.is_none() {
                if let Some(status) = guard.child.try_wait().context("try_wait process")? {
                    guard.exit_code = Some(status.code().unwrap_or(-1));
                }
            }

            out.push(serde_json::json!({
                "process_id": id,
                "command": guard.command.clone(),
                "args": guard.args.clone(),
                "alive": guard.exit_code.is_none(),
                "exit_code": guard.exit_code,
                "uptime_secs": guard.started_at.elapsed().as_secs(),
            }));
        }

        Ok(serde_json::Value::Array(out).to_string())
    }
}
