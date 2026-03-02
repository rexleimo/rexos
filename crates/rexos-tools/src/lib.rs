use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use anyhow::{bail, Context};
use base64::Engine as _;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

use rexos_llm::openai_compat::{ToolDefinition, ToolFunctionDefinition};

const BROWSER_BRIDGE_SCRIPT: &str = include_str!("browser_bridge.py");
static BROWSER_BRIDGE_PATH: OnceLock<PathBuf> = OnceLock::new();

mod browser_cdp;

#[derive(Debug, Clone)]
pub struct Toolset {
    workspace_root: PathBuf,
    http: reqwest::Client,
    browser: Arc<tokio::sync::Mutex<Option<BrowserSession>>>,
    processes: Arc<tokio::sync::Mutex<ProcessManager>>,
}

const PROCESS_MAX_PROCESSES: usize = 5;
const PROCESS_OUTPUT_MAX_BYTES: usize = 200_000;

struct ProcessManager {
    processes: HashMap<String, Arc<tokio::sync::Mutex<ProcessEntry>>>,
}

impl std::fmt::Debug for ProcessManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcessManager")
            .field("processes", &self.processes.len())
            .finish()
    }
}

impl ProcessManager {
    fn new() -> Self {
        Self {
            processes: HashMap::new(),
        }
    }
}

struct ProcessEntry {
    command: String,
    args: Vec<String>,
    started_at: std::time::Instant,
    exit_code: Option<i32>,
    child: tokio::process::Child,
    stdin: Option<tokio::process::ChildStdin>,
    stdout: Arc<tokio::sync::Mutex<Vec<u8>>>,
    stderr: Arc<tokio::sync::Mutex<Vec<u8>>>,
}

impl Drop for ProcessEntry {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

impl Toolset {
    pub fn new(workspace_root: PathBuf) -> anyhow::Result<Self> {
        let workspace_root = workspace_root.canonicalize().with_context(|| {
            format!("canonicalize workspace root: {}", workspace_root.display())
        })?;
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .build()
            .context("build http client")?;
        Ok(Self {
            workspace_root,
            http,
            browser: Arc::new(tokio::sync::Mutex::new(None)),
            processes: Arc::new(tokio::sync::Mutex::new(ProcessManager::new())),
        })
    }

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        let mut defs = vec![
            fs_read_def(),
            fs_write_def(),
            shell_def(),
            web_fetch_def(),
            browser_navigate_def(),
            browser_click_def(),
            browser_type_def(),
            browser_press_key_def(),
            browser_wait_for_def(),
            browser_read_page_def(),
            browser_screenshot_def(),
            browser_close_def(),
        ];
        defs.extend(compat_tool_defs());
        defs
    }

    pub async fn call(&self, name: &str, arguments_json: &str) -> anyhow::Result<String> {
        match name {
            "fs_read" => {
                let args: FsReadArgs =
                    serde_json::from_str(arguments_json).context("parse fs_read arguments")?;
                self.fs_read(&args.path)
            }
            "file_read" => {
                let args: FileReadArgs =
                    serde_json::from_str(arguments_json).context("parse file_read arguments")?;
                self.fs_read(&args.path)
            }
            "fs_write" => {
                let args: FsWriteArgs =
                    serde_json::from_str(arguments_json).context("parse fs_write arguments")?;
                self.fs_write(&args.path, &args.content)
            }
            "file_write" => {
                let args: FileWriteArgs =
                    serde_json::from_str(arguments_json).context("parse file_write arguments")?;
                self.fs_write(&args.path, &args.content)
            }
            "file_list" => {
                let args: FileListArgs =
                    serde_json::from_str(arguments_json).context("parse file_list arguments")?;
                self.file_list(&args.path)
            }
            "apply_patch" => {
                let args: ApplyPatchArgs =
                    serde_json::from_str(arguments_json).context("parse apply_patch arguments")?;
                self.apply_patch(&args.patch)
            }
            "shell" => {
                let args: ShellArgs =
                    serde_json::from_str(arguments_json).context("parse shell arguments")?;
                self.shell(&args.command, args.timeout_ms).await
            }
            "shell_exec" => {
                let args: ShellExecArgs =
                    serde_json::from_str(arguments_json).context("parse shell_exec arguments")?;
                let timeout_ms = args.timeout_seconds.map(|s| s.saturating_mul(1000));
                self.shell(&args.command, timeout_ms).await
            }
            "docker_exec" => {
                let args: DockerExecArgs =
                    serde_json::from_str(arguments_json).context("parse docker_exec arguments")?;
                self.docker_exec(&args.command).await
            }
            "process_start" => {
                let args: ProcessStartArgs = serde_json::from_str(arguments_json)
                    .context("parse process_start arguments")?;
                self.process_start(&args.command, &args.args).await
            }
            "process_poll" => {
                let args: ProcessPollArgs =
                    serde_json::from_str(arguments_json).context("parse process_poll arguments")?;
                self.process_poll(&args.process_id).await
            }
            "process_write" => {
                let args: ProcessWriteArgs = serde_json::from_str(arguments_json)
                    .context("parse process_write arguments")?;
                self.process_write(&args.process_id, &args.data).await
            }
            "process_kill" => {
                let args: ProcessKillArgs =
                    serde_json::from_str(arguments_json).context("parse process_kill arguments")?;
                self.process_kill(&args.process_id).await
            }
            "process_list" => {
                let _args: serde_json::Value =
                    serde_json::from_str(arguments_json).context("parse process_list arguments")?;
                self.process_list().await
            }
            "web_fetch" => {
                let args: WebFetchArgs =
                    serde_json::from_str(arguments_json).context("parse web_fetch arguments")?;
                self.web_fetch(
                    &args.url,
                    args.timeout_ms,
                    args.max_bytes,
                    args.allow_private,
                )
                .await
            }
            "web_search" => {
                let args: WebSearchArgs =
                    serde_json::from_str(arguments_json).context("parse web_search arguments")?;
                self.web_search(&args.query, args.max_results).await
            }
            "a2a_discover" => {
                let args: A2aDiscoverArgs =
                    serde_json::from_str(arguments_json).context("parse a2a_discover arguments")?;
                self.a2a_discover(&args.url, args.allow_private).await
            }
            "a2a_send" => {
                let args: A2aSendArgs =
                    serde_json::from_str(arguments_json).context("parse a2a_send arguments")?;
                let url = args
                    .agent_url
                    .as_deref()
                    .or(args.url.as_deref())
                    .context("missing agent_url (or url) for a2a_send")?;
                self.a2a_send(
                    url,
                    &args.message,
                    args.session_id.as_deref(),
                    args.allow_private,
                )
                .await
            }
            "image_analyze" => {
                let args: ImageAnalyzeArgs = serde_json::from_str(arguments_json)
                    .context("parse image_analyze arguments")?;
                self.image_analyze(&args.path)
            }
            "location_get" => {
                let _args: serde_json::Value =
                    serde_json::from_str(arguments_json).context("parse location_get arguments")?;
                self.location_get()
            }
            "media_describe" => {
                let args: MediaDescribeArgs = serde_json::from_str(arguments_json)
                    .context("parse media_describe arguments")?;
                self.media_describe(&args.path)
            }
            "media_transcribe" => {
                let args: MediaTranscribeArgs = serde_json::from_str(arguments_json)
                    .context("parse media_transcribe arguments")?;
                self.media_transcribe(&args.path)
            }
            "speech_to_text" => {
                let args: SpeechToTextArgs = serde_json::from_str(arguments_json)
                    .context("parse speech_to_text arguments")?;
                self.speech_to_text(&args.path)
            }
            "text_to_speech" => {
                let args: TextToSpeechArgs = serde_json::from_str(arguments_json)
                    .context("parse text_to_speech arguments")?;
                self.text_to_speech(&args.text, args.path.as_deref())
            }
            "image_generate" => {
                let args: ImageGenerateArgs = serde_json::from_str(arguments_json)
                    .context("parse image_generate arguments")?;
                self.image_generate(&args.prompt, &args.path)
            }
            "canvas_present" => {
                let args: CanvasPresentArgs = serde_json::from_str(arguments_json)
                    .context("parse canvas_present arguments")?;
                self.canvas_present(&args.html, args.title.as_deref())
            }
            "browser_navigate" => {
                let args: BrowserNavigateArgs = serde_json::from_str(arguments_json)
                    .context("parse browser_navigate arguments")?;
                self.browser_navigate(
                    &args.url,
                    args.timeout_ms,
                    args.allow_private,
                    args.headless,
                )
                .await
            }
            "browser_close" => {
                let _args: serde_json::Value = serde_json::from_str(arguments_json)
                    .context("parse browser_close arguments")?;
                self.browser_close().await
            }
            "browser_click" => {
                let args: BrowserClickArgs = serde_json::from_str(arguments_json)
                    .context("parse browser_click arguments")?;
                self.browser_click(&args.selector).await
            }
            "browser_type" => {
                let args: BrowserTypeArgs =
                    serde_json::from_str(arguments_json).context("parse browser_type arguments")?;
                self.browser_type(&args.selector, &args.text).await
            }
            "browser_press_key" => {
                let args: BrowserPressKeyArgs = serde_json::from_str(arguments_json)
                    .context("parse browser_press_key arguments")?;
                self.browser_press_key(args.selector.as_deref(), &args.key)
                    .await
            }
            "browser_wait_for" => {
                let args: BrowserWaitForArgs = serde_json::from_str(arguments_json)
                    .context("parse browser_wait_for arguments")?;
                self.browser_wait_for(
                    args.selector.as_deref(),
                    args.text.as_deref(),
                    args.timeout_ms,
                )
                .await
            }
            "browser_read_page" => {
                let _args: serde_json::Value = serde_json::from_str(arguments_json)
                    .context("parse browser_read_page arguments")?;
                self.browser_read_page().await
            }
            "browser_screenshot" => {
                let args: BrowserScreenshotArgs = serde_json::from_str(arguments_json)
                    .context("parse browser_screenshot arguments")?;
                self.browser_screenshot(args.path.as_deref()).await
            }
            "memory_store" | "memory_recall" => {
                bail!("tool '{name}' is implemented in the runtime, not Toolset")
            }
            "agent_send"
            | "agent_spawn"
            | "agent_list"
            | "agent_kill"
            | "agent_find"
            | "hand_list"
            | "hand_activate"
            | "hand_status"
            | "hand_deactivate"
            | "task_post"
            | "task_claim"
            | "task_complete"
            | "task_list"
            | "event_publish"
            | "schedule_create"
            | "schedule_list"
            | "schedule_delete"
            | "knowledge_add_entity"
            | "knowledge_add_relation"
            | "knowledge_query"
            | "cron_create"
            | "cron_list"
            | "cron_cancel"
            | "channel_send" => {
                bail!("tool '{name}' is implemented in the runtime, not Toolset")
            }
            _ => bail!("unknown tool: {name}"),
        }
    }

    fn fs_read(&self, user_path: &str) -> anyhow::Result<String> {
        let path = self.resolve_workspace_path(user_path)?;

        let meta = std::fs::metadata(&path).with_context(|| format!("stat {}", path.display()))?;
        if meta.len() > 200_000 {
            bail!("file too large: {} bytes", meta.len());
        }

        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))
    }

    fn fs_write(&self, user_path: &str, content: &str) -> anyhow::Result<String> {
        let path = self.resolve_workspace_path_for_write(user_path)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create dirs {}", parent.display()))?;
        }

        std::fs::write(&path, content).with_context(|| format!("write {}", path.display()))?;
        Ok("ok".to_string())
    }

    fn file_list(&self, user_path: &str) -> anyhow::Result<String> {
        let resolved = if user_path.trim() == "." {
            self.workspace_root.clone()
        } else {
            self.resolve_workspace_path(user_path)?
        };

        let mut out = Vec::new();
        for entry in std::fs::read_dir(&resolved)
            .with_context(|| format!("list dir {}", resolved.display()))?
        {
            let entry = entry.context("read dir entry")?;
            let name = entry.file_name().to_string_lossy().to_string();
            let suffix = match entry.file_type() {
                Ok(ft) if ft.is_dir() => "/",
                _ => "",
            };
            out.push(format!("{name}{suffix}"));
        }
        out.sort();
        Ok(out.join("\n"))
    }

    fn apply_patch(&self, patch: &str) -> anyhow::Result<String> {
        let ops = parse_patch(patch).context("parse patch")?;
        let mut result = PatchApplyResult::default();

        for op in ops {
            match op {
                PatchOp::AddFile { path, content } => {
                    let dest = self.resolve_workspace_path_for_write(&path)?;
                    if let Some(parent) = dest.parent() {
                        std::fs::create_dir_all(parent)
                            .with_context(|| format!("create dirs {}", parent.display()))?;
                    }
                    std::fs::write(&dest, content)
                        .with_context(|| format!("write {}", dest.display()))?;
                    result.files_added += 1;
                }
                PatchOp::UpdateFile { path, hunks } => {
                    let dest = self.resolve_workspace_path_for_write(&path)?;
                    let before = std::fs::read_to_string(&dest)
                        .with_context(|| format!("read {}", dest.display()))?;
                    let after = apply_hunks_to_text(&before, &hunks).context("apply hunks")?;
                    std::fs::write(&dest, after)
                        .with_context(|| format!("write {}", dest.display()))?;
                    result.files_updated += 1;
                }
                PatchOp::DeleteFile { path } => {
                    let dest = self.resolve_workspace_path(&path)?;
                    std::fs::remove_file(&dest)
                        .with_context(|| format!("delete {}", dest.display()))?;
                    result.files_deleted += 1;
                }
            }
        }

        Ok(result.summary())
    }

    async fn shell(&self, command: &str, timeout_ms: Option<u64>) -> anyhow::Result<String> {
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

    async fn docker_exec(&self, command: &str) -> anyhow::Result<String> {
        let enabled = std::env::var("REXOS_DOCKER_EXEC_ENABLED")
            .ok()
            .map(|v| v.trim() == "1")
            .unwrap_or(false);
        if !enabled {
            bail!("docker_exec is disabled (set REXOS_DOCKER_EXEC_ENABLED=1 to enable)");
        }

        if command.trim().is_empty() {
            bail!("command is empty");
        }

        let image = std::env::var("REXOS_DOCKER_EXEC_IMAGE")
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

    fn spawn_process_output_reader(
        mut stream: impl tokio::io::AsyncRead + Unpin + Send + 'static,
        buffer: Arc<tokio::sync::Mutex<Vec<u8>>>,
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
                buf.extend_from_slice(&tmp[..n]);
                if buf.len() > PROCESS_OUTPUT_MAX_BYTES {
                    let start = buf.len() - PROCESS_OUTPUT_MAX_BYTES;
                    let tail = buf.split_off(start);
                    *buf = tail;
                }
            }
        });
    }

    fn decode_process_output(bytes: Vec<u8>) -> String {
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

    async fn process_start(&self, command: &str, args: &[String]) -> anyhow::Result<String> {
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

        let stdout_buf = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let stderr_buf = Arc::new(tokio::sync::Mutex::new(Vec::new()));
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

    async fn process_poll(&self, process_id: &str) -> anyhow::Result<String> {
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

        let stdout = {
            let mut buf = stdout_buf.lock().await;
            let bytes = std::mem::take(&mut *buf);
            Self::decode_process_output(bytes)
        };
        let stderr = {
            let mut buf = stderr_buf.lock().await;
            let bytes = std::mem::take(&mut *buf);
            Self::decode_process_output(bytes)
        };

        Ok(serde_json::json!({
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": exit_code,
            "alive": alive,
        })
        .to_string())
    }

    async fn process_write(&self, process_id: &str, data: &str) -> anyhow::Result<String> {
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

    async fn process_kill(&self, process_id: &str) -> anyhow::Result<String> {
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

    async fn process_list(&self) -> anyhow::Result<String> {
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

    async fn web_search(&self, query: &str, max_results: Option<u32>) -> anyhow::Result<String> {
        if query.trim().is_empty() {
            bail!("query is empty");
        }

        let max_results = max_results.unwrap_or(5).clamp(1, 20) as usize;
        let resp = self
            .http
            .get("https://html.duckduckgo.com/html/")
            .query(&[("q", query)])
            .header("User-Agent", "Mozilla/5.0 (compatible; RexOS/0.1)")
            .send()
            .await
            .context("send web_search request")?
            .error_for_status()
            .context("web_search http error")?;

        let body = resp.text().await.context("read web_search body")?;
        let results = parse_ddg_results(&body, max_results);
        if results.is_empty() {
            return Ok(format!("No results found for '{query}'."));
        }

        let mut out = format!("Search results for '{query}':\n\n");
        for (idx, (title, url, snippet)) in results.into_iter().enumerate() {
            out.push_str(&format!(
                "{}. {}\n   URL: {}\n   {}\n\n",
                idx + 1,
                title,
                url,
                snippet
            ));
        }
        Ok(out)
    }

    async fn a2a_discover(&self, url: &str, allow_private: bool) -> anyhow::Result<String> {
        let mut url = reqwest::Url::parse(url).context("parse url")?;
        match url.scheme() {
            "http" | "https" => {}
            _ => bail!("only http/https urls are allowed"),
        }

        let host = url.host_str().context("url missing host")?;
        let port = url.port_or_known_default().context("url missing port")?;

        if !allow_private {
            let ips = resolve_host_ips(host, port)
                .await
                .with_context(|| format!("resolve {host}:{port}"))?;
            for ip in ips {
                if is_forbidden_ip(ip) {
                    bail!("url resolves to loopback/private address: {ip}");
                }
            }
        }

        url.set_path("/.well-known/agent.json");
        url.set_query(None);
        url.set_fragment(None);

        let resp = self
            .http
            .get(url.clone())
            .header("User-Agent", "RexOS/0.1 A2A")
            .send()
            .await
            .context("send a2a_discover request")?;

        if !resp.status().is_success() {
            bail!("a2a_discover http {}", resp.status());
        }

        let bytes = resp
            .bytes()
            .await
            .context("read a2a_discover response body")?;
        if bytes.len() > 200_000 {
            bail!("agent card too large: {} bytes", bytes.len());
        }

        let v: serde_json::Value =
            serde_json::from_slice(&bytes).context("parse agent card json")?;
        Ok(serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string()))
    }

    async fn a2a_send(
        &self,
        agent_url: &str,
        message: &str,
        session_id: Option<&str>,
        allow_private: bool,
    ) -> anyhow::Result<String> {
        if message.trim().is_empty() {
            bail!("message is empty");
        }

        let url = reqwest::Url::parse(agent_url).context("parse agent_url")?;
        match url.scheme() {
            "http" | "https" => {}
            _ => bail!("only http/https urls are allowed"),
        }

        let host = url.host_str().context("url missing host")?;
        let port = url.port_or_known_default().context("url missing port")?;

        if !allow_private {
            let ips = resolve_host_ips(host, port)
                .await
                .with_context(|| format!("resolve {host}:{port}"))?;
            for ip in ips {
                if is_forbidden_ip(ip) {
                    bail!("url resolves to loopback/private address: {ip}");
                }
            }
        }

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tasks/send",
            "params": {
                "message": {
                    "role": "user",
                    "parts": [{ "type": "text", "text": message }]
                },
                "sessionId": session_id,
            }
        });

        let resp = self
            .http
            .post(url.clone())
            .header("User-Agent", "RexOS/0.1 A2A")
            .json(&request)
            .send()
            .await
            .context("send a2a_send request")?;

        if !resp.status().is_success() {
            bail!("a2a_send http {}", resp.status());
        }

        let v: serde_json::Value = resp.json().await.context("parse a2a_send response")?;
        if let Some(result) = v.get("result") {
            return Ok(serde_json::to_string_pretty(result).unwrap_or_else(|_| result.to_string()));
        }
        if let Some(err) = v.get("error") {
            bail!("a2a_send error: {err}");
        }
        bail!("invalid a2a_send response")
    }

    async fn web_fetch(
        &self,
        url: &str,
        timeout_ms: Option<u64>,
        max_bytes: Option<u64>,
        allow_private: bool,
    ) -> anyhow::Result<String> {
        let url = reqwest::Url::parse(url).context("parse url")?;
        match url.scheme() {
            "http" | "https" => {}
            _ => bail!("only http/https urls are allowed"),
        }

        let host = url.host_str().context("url missing host")?;
        let port = url.port_or_known_default().context("url missing port")?;

        if !allow_private {
            let ips = resolve_host_ips(host, port)
                .await
                .with_context(|| format!("resolve {host}:{port}"))?;
            for ip in ips {
                if is_forbidden_ip(ip) {
                    bail!("url resolves to loopback/private address: {ip}");
                }
            }
        }

        let timeout = Duration::from_millis(timeout_ms.unwrap_or(20_000));
        let max_bytes = max_bytes.unwrap_or(200_000) as usize;

        let resp = tokio::time::timeout(timeout, self.http.get(url.clone()).send())
            .await
            .context("web_fetch timed out")?
            .context("send request")?;

        let status = resp.status().as_u16();
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let bytes = tokio::time::timeout(timeout, resp.bytes())
            .await
            .context("web_fetch timed out")?
            .context("read response body")?;

        let truncated = bytes.len() > max_bytes;
        let slice = if truncated {
            &bytes[..max_bytes]
        } else {
            &bytes
        };
        let body = String::from_utf8_lossy(slice).to_string();

        Ok(serde_json::json!({
            "status": status,
            "content_type": content_type,
            "body": body,
            "truncated": truncated,
            "bytes": slice.len(),
        })
        .to_string())
    }

    fn image_analyze(&self, user_path: &str) -> anyhow::Result<String> {
        let path = self.resolve_workspace_path(user_path)?;
        let meta = std::fs::metadata(&path).with_context(|| format!("stat {}", path.display()))?;
        if meta.len() > 10_000_000 {
            bail!("image too large: {} bytes", meta.len());
        }

        let bytes = std::fs::read(&path).with_context(|| format!("read {}", path.display()))?;
        let Some((format, width, height)) = detect_image_format_and_dimensions(&bytes) else {
            bail!("unsupported image format (expected png/jpeg/gif)");
        };

        Ok(serde_json::json!({
            "path": user_path,
            "format": format,
            "width": width,
            "height": height,
            "bytes": bytes.len(),
        })
        .to_string())
    }

    fn location_get(&self) -> anyhow::Result<String> {
        let tz = std::env::var("TZ").ok().filter(|v| !v.trim().is_empty());
        let lang = std::env::var("LANG").ok().filter(|v| !v.trim().is_empty());
        let lc_all = std::env::var("LC_ALL")
            .ok()
            .filter(|v| !v.trim().is_empty());

        Ok(serde_json::json!({
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "tz": tz,
            "lang": lang,
            "lc_all": lc_all,
            "geolocation": null,
            "note": "Exact geolocation is not available; RexOS only reports environment metadata.",
        })
        .to_string())
    }

    fn media_describe(&self, user_path: &str) -> anyhow::Result<String> {
        let path = self.resolve_workspace_path(user_path)?;
        let meta = std::fs::metadata(&path).with_context(|| format!("stat {}", path.display()))?;
        if meta.len() > 200_000_000 {
            bail!("media too large: {} bytes", meta.len());
        }

        let ext = path
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        let kind = match ext.as_str() {
            "wav" | "mp3" | "flac" | "ogg" | "m4a" | "aac" | "opus" => "audio",
            "mp4" | "mov" | "mkv" | "webm" | "avi" => "video",
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" => "image",
            "txt" | "md" | "srt" | "vtt" => "text",
            _ => "unknown",
        };

        Ok(serde_json::json!({
            "path": user_path,
            "bytes": meta.len(),
            "kind": kind,
            "ext": if ext.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(ext) },
        })
        .to_string())
    }

    fn media_transcribe(&self, user_path: &str) -> anyhow::Result<String> {
        let path = self.resolve_workspace_path(user_path)?;
        let meta = std::fs::metadata(&path).with_context(|| format!("stat {}", path.display()))?;
        if meta.len() > 2_000_000 {
            bail!("transcript too large: {} bytes", meta.len());
        }

        let ext = path
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        match ext.as_str() {
            "txt" | "md" | "srt" | "vtt" => {}
            _ => bail!("media_transcribe currently supports text transcripts (.txt/.md/.srt/.vtt)"),
        }

        let raw =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let text = raw.trim_end_matches(&['\r', '\n'][..]).to_string();

        Ok(serde_json::json!({
            "path": user_path,
            "text": text,
        })
        .to_string())
    }

    fn speech_to_text(&self, user_path: &str) -> anyhow::Result<String> {
        let out = self.media_transcribe(user_path)?;
        let v: serde_json::Value =
            serde_json::from_str(&out).context("parse media_transcribe output")?;
        let text = v
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        Ok(serde_json::json!({
            "path": user_path,
            "transcript": text,
            "text": v.get("text").cloned().unwrap_or(serde_json::Value::Null),
            "note": "MVP: speech_to_text currently supports transcript files (.txt/.md/.srt/.vtt).",
        })
        .to_string())
    }

    fn text_to_speech(&self, text: &str, path: Option<&str>) -> anyhow::Result<String> {
        if text.trim().is_empty() {
            bail!("text is empty");
        }

        let rel = path.unwrap_or(".rexos/audio/tts.wav");
        let out_path = self.resolve_workspace_path_for_write(rel)?;
        if out_path.extension().and_then(|x| x.to_str()).unwrap_or("") != "wav" {
            bail!("text_to_speech currently only supports .wav output paths");
        }

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create dirs {}", parent.display()))?;
        }

        let sample_rate: u32 = 16_000;
        let duration_ms: u32 = 300;
        let num_samples = (sample_rate as usize)
            .saturating_mul(duration_ms as usize)
            .saturating_div(1000);
        let frequency_hz: f32 = 440.0;
        let amplitude: f32 = 0.20;

        let data_size = num_samples.saturating_mul(2);
        let riff_size = 36u32.saturating_add(data_size as u32);

        let mut bytes = Vec::with_capacity(44 + data_size);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&riff_size.to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk size
        bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
        bytes.extend_from_slice(&1u16.to_le_bytes()); // channels
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        let byte_rate = sample_rate.saturating_mul(2);
        bytes.extend_from_slice(&byte_rate.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes()); // block align
        bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&(data_size as u32).to_le_bytes());

        for n in 0..num_samples {
            let t = n as f32 / sample_rate as f32;
            let s = (2.0 * std::f32::consts::PI * frequency_hz * t).sin();
            let sample = (s * amplitude * i16::MAX as f32) as i16;
            bytes.extend_from_slice(&sample.to_le_bytes());
        }

        std::fs::write(&out_path, &bytes)
            .with_context(|| format!("write {}", out_path.display()))?;

        Ok(serde_json::json!({
            "path": rel,
            "format": "wav",
            "bytes": bytes.len(),
            "note": "MVP: generates a short WAV tone (placeholder for real TTS).",
        })
        .to_string())
    }

    fn image_generate(&self, prompt: &str, user_path: &str) -> anyhow::Result<String> {
        if prompt.trim().is_empty() {
            bail!("prompt is empty");
        }

        let out_path = self.resolve_workspace_path_for_write(user_path)?;
        if out_path.extension().and_then(|x| x.to_str()).unwrap_or("") != "svg" {
            bail!("only svg output is supported for now (use a .svg path)");
        }

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create dirs {}", parent.display()))?;
        }

        let escaped = escape_xml_text(prompt);
        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="800" height="450" viewBox="0 0 800 450"><rect width="100%" height="100%" fill="#0b1020"/><text x="40" y="120" fill="#e2e8f0" font-size="48" font-family="Inter, system-ui, -apple-system, Segoe UI, Roboto, Arial">{escaped}</text></svg>"##
        );

        std::fs::write(&out_path, svg).with_context(|| format!("write {}", out_path.display()))?;

        Ok(serde_json::json!({
            "path": user_path,
            "format": "svg",
        })
        .to_string())
    }

    fn canvas_present(&self, html: &str, title: Option<&str>) -> anyhow::Result<String> {
        let title = title
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .unwrap_or("Canvas");

        let sanitized = sanitize_canvas_html(html, 512 * 1024)?;

        let canvas_id = uuid::Uuid::new_v4().to_string();
        let rel = format!("output/canvas_{canvas_id}.html");
        let out_path = self.resolve_workspace_path_for_write(&rel)?;
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create dirs {}", parent.display()))?;
        }

        let safe_title = escape_xml_text(title);
        let full = format!(
            "<!DOCTYPE html>\n<html>\n<head><meta charset=\"utf-8\"><title>{safe_title}</title></head>\n<body>\n{sanitized}\n</body>\n</html>\n"
        );

        std::fs::write(&out_path, &full)
            .with_context(|| format!("write {}", out_path.display()))?;

        Ok(serde_json::json!({
            "canvas_id": canvas_id,
            "title": title,
            "saved_to": rel,
            "size_bytes": full.len(),
        })
        .to_string())
    }

    async fn browser_navigate(
        &self,
        url: &str,
        _timeout_ms: Option<u64>,
        allow_private: bool,
        headless: Option<bool>,
    ) -> anyhow::Result<String> {
        let url = reqwest::Url::parse(url).context("parse url")?;
        match url.scheme() {
            "http" | "https" => {}
            _ => bail!("only http/https urls are allowed"),
        }

        let host = url.host_str().context("url missing host")?;
        let port = url.port_or_known_default().context("url missing port")?;

        if !allow_private {
            let ips = resolve_host_ips(host, port)
                .await
                .with_context(|| format!("resolve {host}:{port}"))?;
            for ip in ips {
                if is_forbidden_ip(ip) {
                    bail!("url resolves to loopback/private address: {ip}");
                }
            }
        }

        let backend = browser_backend_default();

        let mut guard = self.browser.lock().await;
        if guard.is_none() {
            let headless = headless.unwrap_or_else(browser_headless_default);
            let session = match backend {
                BrowserBackend::Cdp => {
                    let s = browser_cdp::CdpBrowserSession::connect_or_launch(
                        self.http.clone(),
                        headless,
                        allow_private,
                    )
                    .await?;
                    BrowserSession::Cdp(s)
                }
                BrowserBackend::Playwright => {
                    BrowserSession::Playwright(PlaywrightBrowserSession::spawn(
                        headless,
                        allow_private,
                    )
                    .await?)
                }
            };
            *guard = Some(session);
        } else {
            let session = guard.as_ref().expect("checked none");
            if session.backend() != backend {
                bail!(
                    "browser session already started with backend={:?}; call browser_close before switching to backend={:?}",
                    session.backend(),
                    backend
                );
            }

            if let Some(requested) = headless {
                let session_headless = session.headless();
                if session_headless != requested {
                    bail!(
                        "browser session already started with headless={session_headless}; call browser_close before starting a new session with headless={requested}"
                    );
                }
            }
        }

        let session = guard.as_mut().expect("set above");
        session.set_allow_private(allow_private);
        let out = session.navigate(url.as_str()).await?;
        Ok(out.to_string())
    }

    async fn browser_close(&self) -> anyhow::Result<String> {
        let mut guard = self.browser.lock().await;
        if let Some(mut session) = guard.take() {
            session.close().await;
        }
        Ok("ok".to_string())
    }

    async fn browser_click(&self, selector: &str) -> anyhow::Result<String> {
        let mut guard = self.browser.lock().await;
        let session = guard
            .as_mut()
            .context("browser session not started; call browser_navigate first")?;
        let out = session.click(selector).await?;
        if let Some(url) = out.get("url").and_then(|v| v.as_str()) {
            ensure_browser_url_allowed(url, session.allow_private()).await?;
        }
        Ok(out.to_string())
    }

    async fn browser_type(&self, selector: &str, text: &str) -> anyhow::Result<String> {
        let mut guard = self.browser.lock().await;
        let session = guard
            .as_mut()
            .context("browser session not started; call browser_navigate first")?;
        let out = session.type_text(selector, text).await?;
        Ok(out.to_string())
    }

    async fn browser_press_key(&self, selector: Option<&str>, key: &str) -> anyhow::Result<String> {
        let mut guard = self.browser.lock().await;
        let session = guard
            .as_mut()
            .context("browser session not started; call browser_navigate first")?;
        let out = session.press_key(selector, key).await?;
        if let Some(url) = out.get("url").and_then(|v| v.as_str()) {
            ensure_browser_url_allowed(url, session.allow_private()).await?;
        }
        Ok(out.to_string())
    }

    async fn browser_wait_for(
        &self,
        selector: Option<&str>,
        text: Option<&str>,
        timeout_ms: Option<u64>,
    ) -> anyhow::Result<String> {
        if selector.unwrap_or("").trim().is_empty() && text.unwrap_or("").trim().is_empty() {
            bail!("browser_wait_for requires selector or text");
        }

        let mut guard = self.browser.lock().await;
        let session = guard
            .as_mut()
            .context("browser session not started; call browser_navigate first")?;
        let out = session.wait_for(selector, text, timeout_ms).await?;
        if let Some(url) = out.get("url").and_then(|v| v.as_str()) {
            ensure_browser_url_allowed(url, session.allow_private()).await?;
        }
        Ok(out.to_string())
    }

    async fn browser_read_page(&self) -> anyhow::Result<String> {
        let mut guard = self.browser.lock().await;
        let session = guard
            .as_mut()
            .context("browser session not started; call browser_navigate first")?;
        let out = session.read_page().await?;
        if let Some(url) = out.get("url").and_then(|v| v.as_str()) {
            ensure_browser_url_allowed(url, session.allow_private()).await?;
        }
        Ok(out.to_string())
    }

    async fn browser_screenshot(&self, path: Option<&str>) -> anyhow::Result<String> {
        let mut guard = self.browser.lock().await;
        let session = guard
            .as_mut()
            .context("browser session not started; call browser_navigate first")?;
        let data = session.screenshot().await?;
        if let Some(url) = data.get("url").and_then(|v| v.as_str()) {
            ensure_browser_url_allowed(url, session.allow_private()).await?;
        }

        let b64 = data
            .get("image_base64")
            .and_then(|v| v.as_str())
            .context("screenshot response missing image_base64")?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .context("decode screenshot base64")?;

        let rel = path.unwrap_or(".rexos/browser/screenshot.png");
        let out_path = self.resolve_workspace_path_for_write(rel)?;
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create dirs {}", parent.display()))?;
        }
        std::fs::write(&out_path, bytes)
            .with_context(|| format!("write {}", out_path.display()))?;

        Ok(serde_json::json!({
            "status": "ok",
            "path": rel,
            "url": data.get("url").cloned().unwrap_or(serde_json::Value::Null),
        })
        .to_string())
    }

    fn resolve_workspace_path(&self, user_path: &str) -> anyhow::Result<PathBuf> {
        let rel = validate_relative_path(user_path)?;
        let candidate = self.workspace_root.join(&rel);
        self.ensure_no_symlink_escape(&rel)?;
        Ok(candidate)
    }

    fn resolve_workspace_path_for_write(&self, user_path: &str) -> anyhow::Result<PathBuf> {
        let rel = validate_relative_path(user_path)?;
        // For writes, forbid writing to an existing symlink and forbid any symlink components.
        self.ensure_no_symlink_escape(&rel)?;
        let candidate = self.workspace_root.join(&rel);
        if candidate.exists() {
            let ft = std::fs::symlink_metadata(&candidate)?.file_type();
            if ft.is_symlink() {
                bail!("path is a symlink");
            }
        }
        Ok(candidate)
    }

    fn ensure_no_symlink_escape(&self, rel: &Path) -> anyhow::Result<()> {
        let mut cur = self.workspace_root.clone();
        for comp in rel.components() {
            if let Component::Normal(seg) = comp {
                cur.push(seg);
                if cur.exists() {
                    let ft = std::fs::symlink_metadata(&cur)
                        .with_context(|| format!("stat {}", cur.display()))?
                        .file_type();
                    if ft.is_symlink() {
                        bail!("symlinks are not allowed in workspace paths");
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
struct BridgeResponse {
    success: bool,
    #[serde(default)]
    data: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<String>,
}

impl BridgeResponse {
    fn into_data(self) -> anyhow::Result<serde_json::Value> {
        if self.success {
            return Ok(self
                .data
                .unwrap_or_else(|| serde_json::json!({ "status": "ok" })));
        }
        bail!(
            "browser bridge error: {}",
            self.error.unwrap_or_else(|| "unknown error".to_string())
        )
    }

    fn into_tool_output(self) -> anyhow::Result<String> {
        Ok(self.into_data()?.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BrowserBackend {
    Cdp,
    Playwright,
}

fn browser_backend_default() -> BrowserBackend {
    if let Ok(v) = std::env::var("REXOS_BROWSER_BACKEND") {
        match v.trim().to_ascii_lowercase().as_str() {
            "cdp" | "native" | "chromium" => return BrowserBackend::Cdp,
            "playwright" | "bridge" | "python" => return BrowserBackend::Playwright,
            _ => {}
        }
    }
    BrowserBackend::Cdp
}

struct PlaywrightBrowserSession {
    headless: bool,
    allow_private: bool,
    child: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
}

impl std::fmt::Debug for PlaywrightBrowserSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlaywrightBrowserSession")
            .field("headless", &self.headless)
            .field("allow_private", &self.allow_private)
            .finish_non_exhaustive()
    }
}

impl PlaywrightBrowserSession {
    async fn spawn(headless: bool, allow_private: bool) -> anyhow::Result<Self> {
        let python = browser_python_exe();
        let script_path = browser_bridge_script_path()?;

        let mut cmd = tokio::process::Command::new(python);
        cmd.arg("-u").arg(script_path);
        if headless {
            cmd.arg("--headless");
        } else {
            cmd.arg("--no-headless");
        }
        cmd.args(["--width", "1280", "--height", "720", "--timeout", "30"]);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        sandbox_python_env(&mut cmd);

        let mut child = cmd.spawn().context("spawn browser bridge")?;
        let stdin = child.stdin.take().context("capture bridge stdin")?;
        let stdout = child.stdout.take().context("capture bridge stdout")?;

        let mut session = Self {
            headless,
            allow_private,
            child,
            stdin,
            stdout: BufReader::new(stdout),
        };

        let ready = session.read_response().await.context("bridge ready")?;
        let _ = ready.into_data()?;

        Ok(session)
    }

    async fn send(&mut self, cmd: serde_json::Value) -> anyhow::Result<BridgeResponse> {
        let line = serde_json::to_string(&cmd).context("encode bridge command")?;

        tokio::time::timeout(Duration::from_secs(30), async {
            self.stdin
                .write_all(line.as_bytes())
                .await
                .context("write bridge stdin")?;
            self.stdin
                .write_all(b"\n")
                .await
                .context("write bridge newline")?;
            self.stdin.flush().await.context("flush bridge stdin")?;
            anyhow::Ok(())
        })
        .await
        .context("browser bridge timed out")??;

        self.read_response().await
    }

    async fn read_response(&mut self) -> anyhow::Result<BridgeResponse> {
        let mut line = String::new();
        let n = tokio::time::timeout(Duration::from_secs(30), self.stdout.read_line(&mut line))
            .await
            .context("browser bridge timed out")?
            .context("read bridge stdout")?;

        if n == 0 {
            bail!("browser bridge closed unexpectedly");
        }

        serde_json::from_str(line.trim()).context("parse bridge response")
    }

    async fn kill(&mut self) {
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
    }
}

impl Drop for PlaywrightBrowserSession {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

enum BrowserSession {
    Cdp(browser_cdp::CdpBrowserSession),
    Playwright(PlaywrightBrowserSession),
}

impl std::fmt::Debug for BrowserSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cdp(s) => f
                .debug_struct("BrowserSession")
                .field("backend", &"cdp")
                .field("headless", &s.headless)
                .field("allow_private", &s.allow_private)
                .finish(),
            Self::Playwright(s) => f
                .debug_struct("BrowserSession")
                .field("backend", &"playwright")
                .field("headless", &s.headless)
                .field("allow_private", &s.allow_private)
                .finish(),
        }
    }
}

impl BrowserSession {
    fn backend(&self) -> BrowserBackend {
        match self {
            Self::Cdp(_) => BrowserBackend::Cdp,
            Self::Playwright(_) => BrowserBackend::Playwright,
        }
    }

    fn headless(&self) -> bool {
        match self {
            Self::Cdp(s) => s.headless,
            Self::Playwright(s) => s.headless,
        }
    }

    fn allow_private(&self) -> bool {
        match self {
            Self::Cdp(s) => s.allow_private,
            Self::Playwright(s) => s.allow_private,
        }
    }

    fn set_allow_private(&mut self, allow_private: bool) {
        match self {
            Self::Cdp(s) => s.allow_private = allow_private,
            Self::Playwright(s) => s.allow_private = allow_private,
        }
    }

    async fn navigate(&mut self, url: &str) -> anyhow::Result<serde_json::Value> {
        match self {
            Self::Cdp(s) => {
                let mut v = s.navigate(url).await?;
                if let Some(obj) = v.as_object_mut() {
                    obj.insert("headless".to_string(), serde_json::Value::Bool(s.headless));
                }
                Ok(v)
            }
            Self::Playwright(s) => Ok(s
                .send(serde_json::json!({
                    "action": "Navigate",
                    "url": url,
                }))
                .await?
                .into_data()
                .map(|mut v| {
                    if let Some(obj) = v.as_object_mut() {
                        obj.insert("headless".to_string(), serde_json::Value::Bool(s.headless));
                    }
                    v
                })?),
        }
    }

    async fn click(&mut self, selector: &str) -> anyhow::Result<serde_json::Value> {
        match self {
            Self::Cdp(s) => s.click(selector).await,
            Self::Playwright(s) => Ok(s
                .send(serde_json::json!({
                    "action": "Click",
                    "selector": selector,
                }))
                .await?
                .into_data()?),
        }
    }

    async fn type_text(&mut self, selector: &str, text: &str) -> anyhow::Result<serde_json::Value> {
        match self {
            Self::Cdp(s) => s.type_text(selector, text).await,
            Self::Playwright(s) => Ok(s
                .send(serde_json::json!({
                    "action": "Type",
                    "selector": selector,
                    "text": text,
                }))
                .await?
                .into_data()?),
        }
    }

    async fn press_key(
        &mut self,
        selector: Option<&str>,
        key: &str,
    ) -> anyhow::Result<serde_json::Value> {
        match self {
            Self::Cdp(s) => s.press_key(selector, key).await,
            Self::Playwright(s) => {
                let mut cmd = serde_json::json!({
                    "action": "PressKey",
                    "key": key,
                });
                if let Some(sel) = selector {
                    cmd["selector"] = serde_json::Value::String(sel.to_string());
                }
                Ok(s.send(cmd).await?.into_data()?)
            }
        }
    }

    async fn wait_for(
        &mut self,
        selector: Option<&str>,
        text: Option<&str>,
        timeout_ms: Option<u64>,
    ) -> anyhow::Result<serde_json::Value> {
        match self {
            Self::Cdp(s) => s.wait_for(selector, text, timeout_ms).await,
            Self::Playwright(s) => {
                let mut cmd = serde_json::json!({ "action": "WaitFor" });
                if let Some(selector) = selector {
                    cmd["selector"] = serde_json::Value::String(selector.to_string());
                }
                if let Some(text) = text {
                    cmd["text"] = serde_json::Value::String(text.to_string());
                }
                if let Some(timeout_ms) = timeout_ms {
                    cmd["timeout_ms"] = serde_json::Value::Number(timeout_ms.into());
                }
                Ok(s.send(cmd).await?.into_data()?)
            }
        }
    }

    async fn read_page(&mut self) -> anyhow::Result<serde_json::Value> {
        match self {
            Self::Cdp(s) => s.read_page().await,
            Self::Playwright(s) => Ok(s
                .send(serde_json::json!({ "action": "ReadPage" }))
                .await?
                .into_data()?),
        }
    }

    async fn screenshot(&mut self) -> anyhow::Result<serde_json::Value> {
        match self {
            Self::Cdp(s) => s.screenshot().await,
            Self::Playwright(s) => Ok(s
                .send(serde_json::json!({ "action": "Screenshot" }))
                .await?
                .into_data()?),
        }
    }

    async fn close(&mut self) {
        match self {
            Self::Cdp(s) => {
                s.close().await;
            }
            Self::Playwright(s) => {
                let _ = s.send(serde_json::json!({ "action": "Close" })).await;
                s.kill().await;
            }
        }
    }
}

fn browser_headless_default() -> bool {
    if let Ok(v) = std::env::var("REXOS_BROWSER_HEADLESS") {
        match v.trim().to_ascii_lowercase().as_str() {
            "0" | "false" | "no" | "off" => return false,
            "1" | "true" | "yes" | "on" => return true,
            _ => {}
        }
    }
    true
}

fn browser_python_exe() -> String {
    if let Ok(v) = std::env::var("REXOS_BROWSER_PYTHON") {
        if !v.trim().is_empty() {
            return v;
        }
    }
    if cfg!(windows) {
        "python".to_string()
    } else {
        "python3".to_string()
    }
}

fn browser_bridge_script_path() -> anyhow::Result<PathBuf> {
    if let Ok(v) = std::env::var("REXOS_BROWSER_BRIDGE_PATH") {
        let p = PathBuf::from(v);
        if p.exists() {
            return Ok(p);
        }
        bail!("REXOS_BROWSER_BRIDGE_PATH does not exist: {}", p.display());
    }

    if let Some(p) = BROWSER_BRIDGE_PATH.get() {
        return Ok(p.clone());
    }

    let dir = std::env::temp_dir().join("rexos");
    std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let path = dir.join("browser_bridge.py");
    std::fs::write(&path, BROWSER_BRIDGE_SCRIPT)
        .with_context(|| format!("write {}", path.display()))?;
    let _ = BROWSER_BRIDGE_PATH.set(path.clone());
    Ok(path)
}

fn sandbox_python_env(cmd: &mut tokio::process::Command) {
    cmd.env_clear();

    if let Ok(path) = std::env::var("PATH") {
        cmd.env("PATH", path);
    }

    cmd.env("PYTHONIOENCODING", "utf-8");

    if cfg!(windows) {
        for key in [
            "SystemRoot",
            "USERPROFILE",
            "TEMP",
            "TMP",
            "APPDATA",
            "LOCALAPPDATA",
        ] {
            if let Ok(v) = std::env::var(key) {
                cmd.env(key, v);
            }
        }
    } else {
        for key in ["HOME", "USER", "TMPDIR", "XDG_CACHE_HOME"] {
            if let Ok(v) = std::env::var(key) {
                cmd.env(key, v);
            }
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct FsReadArgs {
    path: String,
}

#[derive(Debug, serde::Deserialize)]
struct FileReadArgs {
    path: String,
}

#[derive(Debug, serde::Deserialize)]
struct FsWriteArgs {
    path: String,
    content: String,
}

#[derive(Debug, serde::Deserialize)]
struct FileWriteArgs {
    path: String,
    content: String,
}

#[derive(Debug, serde::Deserialize)]
struct FileListArgs {
    path: String,
}

#[derive(Debug, serde::Deserialize)]
struct ApplyPatchArgs {
    patch: String,
}

#[derive(Debug, serde::Deserialize)]
struct ShellArgs {
    command: String,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[derive(Debug, serde::Deserialize)]
struct ShellExecArgs {
    command: String,
    #[serde(default)]
    timeout_seconds: Option<u64>,
}

#[derive(Debug, serde::Deserialize)]
struct DockerExecArgs {
    command: String,
}

#[derive(Debug, serde::Deserialize)]
struct ProcessStartArgs {
    command: String,
    #[serde(default)]
    args: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct ProcessPollArgs {
    process_id: String,
}

#[derive(Debug, serde::Deserialize)]
struct ProcessWriteArgs {
    process_id: String,
    data: String,
}

#[derive(Debug, serde::Deserialize)]
struct ProcessKillArgs {
    process_id: String,
}

#[derive(Debug, serde::Deserialize)]
struct WebFetchArgs {
    url: String,
    #[serde(default)]
    timeout_ms: Option<u64>,
    #[serde(default)]
    max_bytes: Option<u64>,
    #[serde(default)]
    allow_private: bool,
}

#[derive(Debug, serde::Deserialize)]
struct WebSearchArgs {
    query: String,
    #[serde(default)]
    max_results: Option<u32>,
}

#[derive(Debug, serde::Deserialize)]
struct A2aDiscoverArgs {
    url: String,
    #[serde(default)]
    allow_private: bool,
}

#[derive(Debug, serde::Deserialize)]
struct A2aSendArgs {
    #[serde(default)]
    agent_url: Option<String>,
    #[serde(default)]
    url: Option<String>,
    message: String,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    allow_private: bool,
}

#[derive(Debug, serde::Deserialize)]
struct ImageAnalyzeArgs {
    path: String,
}

#[derive(Debug, serde::Deserialize)]
struct MediaDescribeArgs {
    path: String,
}

#[derive(Debug, serde::Deserialize)]
struct MediaTranscribeArgs {
    path: String,
}

#[derive(Debug, serde::Deserialize)]
struct SpeechToTextArgs {
    path: String,
}

#[derive(Debug, serde::Deserialize)]
struct TextToSpeechArgs {
    text: String,
    #[serde(default)]
    path: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct ImageGenerateArgs {
    prompt: String,
    path: String,
}

#[derive(Debug, serde::Deserialize)]
struct CanvasPresentArgs {
    html: String,
    #[serde(default)]
    title: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct BrowserNavigateArgs {
    url: String,
    #[serde(default)]
    timeout_ms: Option<u64>,
    #[serde(default)]
    allow_private: bool,
    #[serde(default)]
    headless: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
struct BrowserClickArgs {
    selector: String,
}

#[derive(Debug, serde::Deserialize)]
struct BrowserTypeArgs {
    selector: String,
    text: String,
}

#[derive(Debug, serde::Deserialize)]
struct BrowserPressKeyArgs {
    key: String,
    #[serde(default)]
    selector: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct BrowserWaitForArgs {
    #[serde(default)]
    selector: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[derive(Debug, serde::Deserialize)]
struct BrowserScreenshotArgs {
    #[serde(default)]
    path: Option<String>,
}

fn validate_relative_path(user_path: &str) -> anyhow::Result<PathBuf> {
    if user_path.trim().is_empty() {
        bail!("path is empty");
    }

    let p = Path::new(user_path);
    if p.is_absolute() {
        bail!("absolute paths are not allowed");
    }

    let mut out = PathBuf::new();
    for comp in p.components() {
        match comp {
            Component::CurDir => {}
            Component::Normal(seg) => out.push(seg),
            Component::ParentDir => bail!("parent traversal is not allowed"),
            Component::RootDir | Component::Prefix(_) => bail!("invalid path"),
        }
    }

    if out.as_os_str().is_empty() {
        bail!("invalid path");
    }
    Ok(out)
}

fn escape_xml_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

fn contains_event_handler_attr(lower: &str) -> bool {
    let bytes = lower.as_bytes();
    for i in 0..bytes.len().saturating_sub(2) {
        if bytes[i] != b'o' || bytes[i + 1] != b'n' {
            continue;
        }

        if i > 0 {
            let prev = bytes[i - 1];
            let ok_boundary =
                prev.is_ascii_whitespace() || matches!(prev, b'<' | b'"' | b'\'' | b'/' | b'=');
            if !ok_boundary {
                continue;
            }
        }

        let mut j = i + 2;
        let mut had_letter = false;
        while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
            had_letter = true;
            j += 1;
        }
        if !had_letter {
            continue;
        }

        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        if j < bytes.len() && bytes[j] == b'=' {
            return true;
        }
    }
    false
}

fn sanitize_canvas_html(html: &str, max_bytes: usize) -> anyhow::Result<String> {
    if html.trim().is_empty() {
        bail!("html is empty");
    }
    if html.len() > max_bytes {
        bail!("html too large: {} bytes (max {})", html.len(), max_bytes);
    }

    let lower = html.to_ascii_lowercase();

    for tag in [
        "<script", "</script", "<iframe", "</iframe", "<object", "</object", "<embed", "</embed",
        "<applet", "</applet",
    ] {
        if lower.contains(tag) {
            bail!("forbidden html tag detected: {tag}");
        }
    }

    if contains_event_handler_attr(&lower) {
        bail!("forbidden event handler attribute detected (on* attributes are not allowed)");
    }

    for scheme in ["javascript:", "vbscript:", "data:text/html"] {
        if lower.contains(scheme) {
            bail!("forbidden url scheme detected: {scheme}");
        }
    }

    Ok(html.to_string())
}

fn detect_image_format_and_dimensions(bytes: &[u8]) -> Option<(&'static str, u32, u32)> {
    if let Some((w, h)) = parse_png_dimensions(bytes) {
        return Some(("png", w, h));
    }
    if let Some((w, h)) = parse_jpeg_dimensions(bytes) {
        return Some(("jpeg", w, h));
    }
    if let Some((w, h)) = parse_gif_dimensions(bytes) {
        return Some(("gif", w, h));
    }
    None
}

fn parse_png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if bytes.len() < 24 {
        return None;
    }
    if bytes.get(0..8)? != SIG {
        return None;
    }
    if bytes.get(12..16)? != b"IHDR" {
        return None;
    }

    let w = u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?);
    let h = u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?);
    Some((w, h))
}

fn parse_gif_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 10 {
        return None;
    }
    if bytes.get(0..6)? != b"GIF87a" && bytes.get(0..6)? != b"GIF89a" {
        return None;
    }
    let w = u16::from_le_bytes(bytes.get(6..8)?.try_into().ok()?) as u32;
    let h = u16::from_le_bytes(bytes.get(8..10)?.try_into().ok()?) as u32;
    Some((w, h))
}

fn parse_jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 4 {
        return None;
    }
    if bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return None;
    }

    let mut i = 2usize;
    while i + 1 < bytes.len() {
        if bytes[i] != 0xFF {
            i += 1;
            continue;
        }

        while i < bytes.len() && bytes[i] == 0xFF {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }

        let marker = bytes[i];
        i += 1;

        if marker == 0xD9 || marker == 0xDA {
            break;
        }

        if i + 1 >= bytes.len() {
            break;
        }
        let seg_len = u16::from_be_bytes([bytes[i], bytes[i + 1]]) as usize;
        i += 2;
        if seg_len < 2 || i + seg_len - 2 > bytes.len() {
            break;
        }

        let is_sof = matches!(
            marker,
            0xC0 | 0xC1
                | 0xC2
                | 0xC3
                | 0xC5
                | 0xC6
                | 0xC7
                | 0xC9
                | 0xCA
                | 0xCB
                | 0xCD
                | 0xCE
                | 0xCF
        );
        if is_sof {
            if seg_len < 7 || i + 4 >= bytes.len() {
                return None;
            }
            let h = u16::from_be_bytes([bytes[i + 1], bytes[i + 2]]) as u32;
            let w = u16::from_be_bytes([bytes[i + 3], bytes[i + 4]]) as u32;
            return Some((w, h));
        }

        i += seg_len - 2;
    }

    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PatchOp {
    AddFile { path: String, content: String },
    UpdateFile { path: String, hunks: Vec<PatchHunk> },
    DeleteFile { path: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PatchHunk {
    old_lines: Vec<String>,
    new_lines: Vec<String>,
}

#[derive(Debug, Default)]
struct PatchApplyResult {
    files_added: u32,
    files_updated: u32,
    files_deleted: u32,
}

impl PatchApplyResult {
    fn summary(&self) -> String {
        let mut parts = Vec::new();
        if self.files_added > 0 {
            parts.push(format!("{} added", self.files_added));
        }
        if self.files_updated > 0 {
            parts.push(format!("{} updated", self.files_updated));
        }
        if self.files_deleted > 0 {
            parts.push(format!("{} deleted", self.files_deleted));
        }
        if parts.is_empty() {
            "No changes applied".to_string()
        } else {
            parts.join(", ")
        }
    }
}

fn parse_patch(input: &str) -> anyhow::Result<Vec<PatchOp>> {
    let lines: Vec<&str> = input.lines().collect();
    let begin = lines
        .iter()
        .position(|l| l.trim() == "*** Begin Patch")
        .context("missing '*** Begin Patch' marker")?;
    let end = lines
        .iter()
        .rposition(|l| l.trim() == "*** End Patch")
        .context("missing '*** End Patch' marker")?;
    if end <= begin {
        bail!("'*** End Patch' must come after '*** Begin Patch'");
    }

    let body = &lines[begin + 1..end];
    let mut ops = Vec::new();
    let mut i = 0usize;

    while i < body.len() {
        let line = body[i].trim();
        if line.is_empty() {
            i += 1;
            continue;
        }

        if let Some(rest) = line.strip_prefix("*** Add File:") {
            let path = rest.trim().to_string();
            if path.is_empty() {
                bail!("empty path in '*** Add File:'");
            }
            i += 1;

            let mut content_lines = Vec::new();
            while i < body.len() && !body[i].trim().starts_with("***") {
                let raw = body[i];
                if let Some(stripped) = raw.strip_prefix('+') {
                    content_lines.push(stripped.to_string());
                } else if raw.trim().is_empty() {
                    content_lines.push(String::new());
                } else {
                    bail!("expected '+' prefix in Add File content, got: {}", raw);
                }
                i += 1;
            }

            ops.push(PatchOp::AddFile {
                path,
                content: content_lines.join("\n"),
            });
            continue;
        }

        if let Some(rest) = line.strip_prefix("*** Update File:") {
            let path = rest.trim().to_string();
            if path.is_empty() {
                bail!("empty path in '*** Update File:'");
            }
            i += 1;

            let mut hunks = Vec::new();
            while i < body.len() && !body[i].trim().starts_with("***") {
                let cur = body[i].trim();
                if cur.starts_with("@@") {
                    i += 1;
                    let mut old_lines = Vec::new();
                    let mut new_lines = Vec::new();
                    while i < body.len()
                        && !body[i].trim().starts_with("@@")
                        && !body[i].trim().starts_with("***")
                    {
                        let hl = body[i];
                        if let Some(stripped) = hl.strip_prefix('-') {
                            old_lines.push(stripped.to_string());
                        } else if let Some(stripped) = hl.strip_prefix('+') {
                            new_lines.push(stripped.to_string());
                        }
                        i += 1;
                    }
                    hunks.push(PatchHunk {
                        old_lines,
                        new_lines,
                    });
                } else {
                    i += 1;
                }
            }

            if hunks.is_empty() {
                bail!("Update File '{path}' has no hunks");
            }

            ops.push(PatchOp::UpdateFile { path, hunks });
            continue;
        }

        if let Some(rest) = line.strip_prefix("*** Delete File:") {
            let path = rest.trim().to_string();
            if path.is_empty() {
                bail!("empty path in '*** Delete File:'");
            }
            i += 1;
            ops.push(PatchOp::DeleteFile { path });
            continue;
        }

        bail!("unknown patch directive: {line}");
    }

    Ok(ops)
}

fn apply_hunks_to_text(before: &str, hunks: &[PatchHunk]) -> anyhow::Result<String> {
    let trailing_newline = before.ends_with('\n');
    let mut lines: Vec<String> = before.lines().map(|l| l.to_string()).collect();

    for hunk in hunks {
        if hunk.old_lines.is_empty() {
            lines.extend(hunk.new_lines.clone());
            continue;
        }

        let mut found = None;
        for idx in 0..=lines.len().saturating_sub(hunk.old_lines.len()) {
            if lines[idx..idx + hunk.old_lines.len()] == hunk.old_lines {
                found = Some(idx);
                break;
            }
        }

        let idx = found.context("hunk target not found in file")?;
        lines.splice(idx..idx + hunk.old_lines.len(), hunk.new_lines.clone());
    }

    let mut out = lines.join("\n");
    if trailing_newline {
        out.push('\n');
    }
    Ok(out)
}

fn parse_ddg_results(html: &str, max: usize) -> Vec<(String, String, String)> {
    let mut results = Vec::new();

    for chunk in html.split("class=\"result__a\"") {
        if results.len() >= max {
            break;
        }
        if !chunk.contains("href=") {
            continue;
        }

        let url = extract_between(chunk, "href=\"", "\"")
            .unwrap_or_default()
            .to_string();

        let actual_url = if url.contains("uddg=") {
            url.split("uddg=")
                .nth(1)
                .and_then(|u| u.split('&').next())
                .map(percent_decode)
                .unwrap_or(url)
        } else {
            url
        };

        let title = extract_between(chunk, ">", "</a>")
            .map(strip_html_tags)
            .unwrap_or_default();

        let snippet = if let Some(start) = chunk.find("class=\"result__snippet\"") {
            let after = &chunk[start..];
            extract_between(after, ">", "</a>")
                .or_else(|| extract_between(after, ">", "</"))
                .map(strip_html_tags)
                .unwrap_or_default()
        } else {
            String::new()
        };

        if !title.is_empty() && !actual_url.is_empty() {
            results.push((title, actual_url, snippet));
        }
    }

    results
}

fn extract_between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let start_idx = text.find(start)? + start.len();
    let remaining = &text[start_idx..];
    let end_idx = remaining.find(end)?;
    Some(&remaining[..end_idx])
}

fn strip_html_tags(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    result
}

fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hi = bytes[i + 1];
                let lo = bytes[i + 2];
                let hex = |b: u8| -> Option<u8> {
                    match b {
                        b'0'..=b'9' => Some(b - b'0'),
                        b'a'..=b'f' => Some(b - b'a' + 10),
                        b'A'..=b'F' => Some(b - b'A' + 10),
                        _ => None,
                    }
                };
                if let (Some(hi), Some(lo)) = (hex(hi), hex(lo)) {
                    out.push((hi * 16 + lo) as char);
                    i += 3;
                } else {
                    out.push('%');
                    i += 1;
                }
            }
            b'+' => {
                out.push(' ');
                i += 1;
            }
            b => {
                out.push(b as char);
                i += 1;
            }
        }
    }
    out
}

fn fs_read_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "fs_read".to_string(),
            description: "Read a UTF-8 text file from the workspace.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path inside the workspace." }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
    }
}

fn compat_tool_defs() -> Vec<ToolDefinition> {
    use serde_json::json;

    let mut defs = Vec::new();

    // Compatibility aliases that map to RexOS primitives.
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "file_read".to_string(),
            description: "Read the contents of a file. Paths are relative to the agent workspace."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "The file path to read" }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "file_write".to_string(),
            description: "Write content to a file. Paths are relative to the agent workspace."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "The file path to write to" },
                    "content": { "type": "string", "description": "The content to write" }
                },
                "required": ["path", "content"],
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "file_list".to_string(),
            description: "List files in a directory. Paths are relative to the agent workspace."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "The directory path to list" }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "apply_patch".to_string(),
            description: "Apply a multi-hunk diff patch to add, update, or delete files."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "patch": { "type": "string", "description": "Patch in *** Begin Patch / *** End Patch format." }
                },
                "required": ["patch"],
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "shell_exec".to_string(),
            description: "Execute a shell command and return its output.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "The command to execute" },
                    "timeout_seconds": { "type": "integer", "description": "Timeout in seconds (default: 30)" }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "web_search".to_string(),
            description: "Search the web and return a short list of results.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "The search query" },
                    "max_results": { "type": "integer", "description": "Maximum number of results to return (default: 5, max: 20)" }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "memory_store".to_string(),
            description: "Persist a key/value pair to shared memory.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "The memory key" },
                    "value": { "type": "string", "description": "The value to store" }
                },
                "required": ["key", "value"],
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "memory_recall".to_string(),
            description: "Recall a value from shared memory.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "The memory key" }
                },
                "required": ["key"],
                "additionalProperties": false
            }),
        },
    });

    // Collaboration/runtime tools (implemented in the agent runtime; persisted in shared memory).
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "agent_spawn".to_string(),
            description: "Create an agent session record (persisted) and return its details."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "agent_id": { "type": "string", "description": "Optional stable agent id. If omitted, RexOS generates one." },
                    "name": { "type": "string", "description": "Optional human-friendly name." },
                    "system_prompt": { "type": "string", "description": "Optional system prompt for the agent session." },
                    "manifest_toml": { "type": "string", "description": "Optional agent manifest (TOML). RexOS will best-effort extract name + system prompt." }
                },
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "agent_list".to_string(),
            description: "List known agent sessions.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "agent_find".to_string(),
            description: "Find agent sessions by id or name (substring match).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query (case-insensitive substring)." }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "agent_kill".to_string(),
            description: "Mark an agent session as killed.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "agent_id": { "type": "string", "description": "Target agent id." }
                },
                "required": ["agent_id"],
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "agent_send".to_string(),
            description: "Send a message to an agent session and return its response.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "agent_id": { "type": "string", "description": "Target agent id." },
                    "message": { "type": "string", "description": "Message to send." }
                },
                "required": ["agent_id", "message"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "task_post".to_string(),
            description: "Post a task into the shared task board.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "Optional stable task id. If omitted, RexOS generates one." },
                    "title": { "type": "string", "description": "Short title." },
                    "description": { "type": "string", "description": "Task description." },
                    "assigned_to": { "type": "string", "description": "Optional assignee agent id." }
                },
                "required": ["title", "description"],
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "task_list".to_string(),
            description: "List tasks (optionally filtered by status).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "status": { "type": "string", "description": "Optional filter: pending | claimed | completed." }
                },
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "task_claim".to_string(),
            description: "Claim the next available pending task.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "agent_id": { "type": "string", "description": "Optional agent id claiming the task." }
                },
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "task_complete".to_string(),
            description: "Mark a task as completed.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "Task id." },
                    "result": { "type": "string", "description": "Completion result summary." }
                },
                "required": ["task_id", "result"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "event_publish".to_string(),
            description: "Publish an event into the shared event log.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "event_type": { "type": "string", "description": "Event type/name." },
                    "payload": { "type": "object", "description": "Optional event payload." }
                },
                "required": ["event_type"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "schedule_create".to_string(),
            description: "Create a schedule entry (persisted).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Optional stable schedule id. If omitted, RexOS generates one." },
                    "description": { "type": "string", "description": "Human-readable description." },
                    "schedule": { "type": "string", "description": "Schedule expression (stored as-is)." },
                    "agent_id": { "type": "string", "description": "Optional agent id to associate with this schedule." },
                    "agent": { "type": "string", "description": "Alias of agent_id (optional)." },
                    "enabled": { "type": "boolean", "description": "Whether this schedule is enabled (default: true)." }
                },
                "required": ["description", "schedule"],
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "schedule_list".to_string(),
            description: "List schedule entries.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "schedule_delete".to_string(),
            description: "Delete a schedule entry by id.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Schedule id." }
                },
                "required": ["id"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "knowledge_add_entity".to_string(),
            description: "Add an entity to the shared knowledge graph.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Optional stable entity id. If omitted, RexOS generates one." },
                    "name": { "type": "string", "description": "Entity name." },
                    "entity_type": { "type": "string", "description": "Entity type (free-form string)." },
                    "properties": { "type": "object", "description": "Optional properties map.", "additionalProperties": true }
                },
                "required": ["name", "entity_type"],
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "knowledge_add_relation".to_string(),
            description: "Add a relation to the shared knowledge graph.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Optional stable relation id. If omitted, RexOS generates one." },
                    "source": { "type": "string", "description": "Source entity id." },
                    "relation": { "type": "string", "description": "Relation type/name." },
                    "target": { "type": "string", "description": "Target entity id." },
                    "properties": { "type": "object", "description": "Optional properties map.", "additionalProperties": true }
                },
                "required": ["source", "relation", "target"],
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "knowledge_query".to_string(),
            description: "Query the shared knowledge graph.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Query string (substring match)." }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "image_analyze".to_string(),
            description: "Analyze an image file in the workspace (basic metadata).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Workspace-relative image path." }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "location_get".to_string(),
            description: "Get environment location metadata (os/arch/tz).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "media_describe".to_string(),
            description: "Describe a media file in the workspace (best-effort metadata)."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Workspace-relative media path." }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "media_transcribe".to_string(),
            description: "Transcribe media into text (currently supports text transcript files)."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Workspace-relative transcript path (.txt/.md/.srt/.vtt)." }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "image_generate".to_string(),
            description: "Generate an image asset from a prompt (currently outputs SVG).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "prompt": { "type": "string", "description": "Image generation prompt." },
                    "path": { "type": "string", "description": "Workspace-relative output path (use .svg)." }
                },
                "required": ["prompt", "path"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "cron_create".to_string(),
            description: "Create a cron/scheduled job record (persisted).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "job_id": { "type": "string", "description": "Optional stable job id. If omitted, RexOS generates one." },
                    "name": { "type": "string", "description": "Job name." },
                    "schedule": { "type": "object", "description": "Schedule payload (stored as-is)." },
                    "action": { "type": "object", "description": "Action payload (stored as-is)." },
                    "delivery": { "type": "object", "description": "Optional delivery payload (stored as-is)." },
                    "one_shot": { "type": "boolean", "description": "If true, job should be considered one-shot (stored)." },
                    "enabled": { "type": "boolean", "description": "Whether this job is enabled (default: true)." }
                },
                "required": ["name", "schedule", "action"],
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "cron_list".to_string(),
            description: "List cron/scheduled job records.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "cron_cancel".to_string(),
            description: "Cancel a cron/scheduled job record by id.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "job_id": { "type": "string", "description": "Job id." }
                },
                "required": ["job_id"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "channel_send".to_string(),
            description:
                "Enqueue an outbound message into the outbox (delivery happens via dispatcher)."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel": { "type": "string", "description": "Channel adapter name (console, webhook)." },
                    "recipient": { "type": "string", "description": "Channel-specific recipient identifier." },
                    "subject": { "type": "string", "description": "Optional subject line (used by some channels)." },
                    "message": { "type": "string", "description": "Message body to send." }
                },
                "required": ["channel", "recipient", "message"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "hand_list".to_string(),
            description:
                "List available Hands (curated autonomous packages) and their activation status."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "hand_activate".to_string(),
            description: "Activate a Hand (spawns a specialized agent instance).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "hand_id": { "type": "string", "description": "Hand id (e.g. 'browser', 'coder')." },
                    "config": { "type": "object", "description": "Optional hand configuration (stored and appended to the hand system prompt)." }
                },
                "required": ["hand_id"],
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "hand_status".to_string(),
            description: "Get status for a Hand by id.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "hand_id": { "type": "string", "description": "Hand id." }
                },
                "required": ["hand_id"],
                "additionalProperties": false
            }),
        },
    });
    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "hand_deactivate".to_string(),
            description: "Deactivate a running Hand instance.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "instance_id": { "type": "string", "description": "Hand instance id returned by hand_activate." }
                },
                "required": ["instance_id"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "a2a_discover".to_string(),
            description: "Discover an external A2A agent by fetching its agent card at `/.well-known/agent.json`.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "Base URL of the remote agent (http/https)." },
                    "allow_private": { "type": "boolean", "description": "Allow loopback/private IPs (default false)." }
                },
                "required": ["url"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "a2a_send".to_string(),
            description: "Send a JSON-RPC `tasks/send` request to an external A2A agent endpoint.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "agent_url": { "type": "string", "description": "Full JSON-RPC endpoint URL (http/https)." },
                    "url": { "type": "string", "description": "Alias for agent_url." },
                    "message": { "type": "string", "description": "Message to send to the remote agent." },
                    "session_id": { "type": "string", "description": "Optional session id for continuity." },
                    "allow_private": { "type": "boolean", "description": "Allow loopback/private IPs (default false)." }
                },
                "required": ["message"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "text_to_speech".to_string(),
            description: "Convert text to speech audio (MVP: writes a short .wav).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text to convert to speech." },
                    "path": { "type": "string", "description": "Workspace-relative output path (use .wav). Optional." },
                    "voice": { "type": "string", "description": "Optional voice name (ignored in MVP)." },
                    "format": { "type": "string", "description": "Optional format (ignored in MVP; only .wav is supported)." }
                },
                "required": ["text"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "speech_to_text".to_string(),
            description: "Transcribe speech/audio into text (MVP: supports transcript files).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Workspace-relative transcript path (.txt/.md/.srt/.vtt)." }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "docker_exec".to_string(),
            description: "Run a command inside a one-shot Docker container with the workspace mounted (disabled by default).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Command to execute inside the container (passed to `sh -lc`)." }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "process_start".to_string(),
            description: "Start a long-running process (REPL/server). Returns a process_id.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Executable to run (e.g. 'python', 'node', 'bash')." },
                    "args": { "type": "array", "items": { "type": "string" }, "description": "Optional command-line args." }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "process_poll".to_string(),
            description: "Drain buffered stdout/stderr from a running process (non-blocking).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "process_id": { "type": "string", "description": "Process id returned by process_start." }
                },
                "required": ["process_id"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "process_write".to_string(),
            description: "Write data to a running process's stdin (appends newline if missing).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "process_id": { "type": "string", "description": "Process id returned by process_start." },
                    "data": { "type": "string", "description": "Data to write to stdin." }
                },
                "required": ["process_id", "data"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "process_kill".to_string(),
            description: "Terminate a running process and clean up resources.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "process_id": { "type": "string", "description": "Process id returned by process_start." }
                },
                "required": ["process_id"],
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "process_list".to_string(),
            description: "List running processes started via process_start.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
    });

    defs.push(ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "canvas_present".to_string(),
            description: "Present sanitized HTML as a canvas artifact (saved to workspace output/).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "html": { "type": "string", "description": "HTML content to present (scripts/event handlers are forbidden)." },
                    "title": { "type": "string", "description": "Optional canvas title." }
                },
                "required": ["html"],
                "additionalProperties": false
            }),
        },
    });

    defs
}

fn fs_write_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "fs_write".to_string(),
            description: "Write a UTF-8 text file to the workspace (creates parent dirs)."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path inside the workspace." },
                    "content": { "type": "string", "description": "Full file contents to write." }
                },
                "required": ["path", "content"],
                "additionalProperties": false
            }),
        },
    }
}

fn shell_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "shell".to_string(),
            description:
                "Run a shell command inside the workspace (bash on Unix, PowerShell on Windows)."
                    .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Command to run." },
                    "timeout_ms": { "type": "integer", "description": "Timeout in milliseconds (default 60000).", "minimum": 1 }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
        },
    }
}

fn web_fetch_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "web_fetch".to_string(),
            description:
                "Fetch a URL via HTTP(S) and return a small response body (SSRF-protected)."
                    .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "HTTP(S) URL to fetch." },
                    "timeout_ms": { "type": "integer", "description": "Timeout in milliseconds (default 20000).", "minimum": 1 },
                    "max_bytes": { "type": "integer", "description": "Maximum bytes to return (default 200000).", "minimum": 1 },
                    "allow_private": { "type": "boolean", "description": "Allow fetching loopback/private IPs (default false)." }
                },
                "required": ["url"],
                "additionalProperties": false
            }),
        },
    }
}

fn browser_navigate_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "browser_navigate".to_string(),
            description: "Navigate the browser to a URL (SSRF-protected by default).".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "HTTP(S) URL to open." },
                    "timeout_ms": { "type": "integer", "description": "Timeout in milliseconds (default 30000).", "minimum": 1 },
                    "allow_private": { "type": "boolean", "description": "Allow loopback/private IPs (default false)." },
                    "headless": { "type": "boolean", "description": "Run the browser in headless mode (default true). Set false to show a GUI window." }
                },
                "required": ["url"],
                "additionalProperties": false
            }),
        },
    }
}

fn browser_click_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "browser_click".to_string(),
            description:
                "Click an element in the browser by CSS selector (or best-effort text fallback)."
                    .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "selector": { "type": "string", "description": "CSS selector (or text fallback) to click." }
                },
                "required": ["selector"],
                "additionalProperties": false
            }),
        },
    }
}

fn browser_type_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "browser_type".to_string(),
            description: "Type into an input element in the browser (fills the field).".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "selector": { "type": "string", "description": "CSS selector for the input element." },
                    "text": { "type": "string", "description": "Text to input." }
                },
                "required": ["selector", "text"],
                "additionalProperties": false
            }),
        },
    }
}

fn browser_press_key_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "browser_press_key".to_string(),
            description: "Press a key in the browser (optionally on a target element).".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Key to press (example: Enter, Escape, ArrowDown, Control+A)." },
                    "selector": { "type": "string", "description": "Optional CSS selector to target before pressing the key." }
                },
                "required": ["key"],
                "additionalProperties": false
            }),
        },
    }
}

fn browser_wait_for_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "browser_wait_for".to_string(),
            description: "Wait for a selector or text to appear on the page.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "selector": { "type": "string", "description": "Optional CSS selector to wait for." },
                    "text": { "type": "string", "description": "Optional visible text to wait for." },
                    "timeout_ms": { "type": "integer", "description": "Optional timeout in milliseconds.", "minimum": 1 }
                },
                "additionalProperties": false
            }),
        },
    }
}

fn browser_read_page_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "browser_read_page".to_string(),
            description: "Read the current page content (title/url/text).".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": [],
                "additionalProperties": false
            }),
        },
    }
}

fn browser_screenshot_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "browser_screenshot".to_string(),
            description: "Take a screenshot and write it to a workspace path.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative output path (default .rexos/browser/screenshot.png)." }
                },
                "required": [],
                "additionalProperties": false
            }),
        },
    }
}

fn browser_close_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "browser_close".to_string(),
            description: "Close the browser session (idempotent).".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": [],
                "additionalProperties": false
            }),
        },
    }
}

async fn resolve_host_ips(host: &str, port: u16) -> anyhow::Result<Vec<IpAddr>> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok(vec![ip]);
    }

    let addrs = tokio::net::lookup_host((host, port))
        .await
        .context("dns lookup")?;

    let mut ips = Vec::new();
    for sa in addrs {
        ips.push(sa.ip());
    }

    if ips.is_empty() {
        bail!("no addresses found");
    }

    ips.sort();
    ips.dedup();
    Ok(ips)
}

async fn ensure_browser_url_allowed(url: &str, allow_private: bool) -> anyhow::Result<()> {
    if allow_private {
        return Ok(());
    }

    let url = match reqwest::Url::parse(url) {
        Ok(v) => v,
        Err(_) => return Ok(()),
    };

    match url.scheme() {
        "http" | "https" => {}
        _ => return Ok(()),
    }

    let Some(host) = url.host_str() else { return Ok(()) };
    let Some(port) = url.port_or_known_default() else { return Ok(()) };

    let ips = resolve_host_ips(host, port)
        .await
        .with_context(|| format!("resolve {host}:{port}"))?;
    for ip in ips {
        if is_forbidden_ip(ip) {
            bail!("url resolves to loopback/private address: {ip}");
        }
    }
    Ok(())
}

fn is_forbidden_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_forbidden_ipv4(v4),
        IpAddr::V6(v6) => is_forbidden_ipv6(v6),
    }
}

fn is_forbidden_ipv4(ip: Ipv4Addr) -> bool {
    if ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_multicast()
        || ip.is_unspecified()
    {
        return true;
    }

    // Carrier-grade NAT: 100.64.0.0/10
    let o = ip.octets();
    if o[0] == 100 && (64..=127).contains(&o[1]) {
        return true;
    }

    false
}

fn is_forbidden_ipv6(ip: Ipv6Addr) -> bool {
    if ip.is_loopback()
        || ip.is_unique_local()
        || ip.is_unicast_link_local()
        || ip.is_multicast()
        || ip.is_unspecified()
    {
        return true;
    }

    // Site-local (deprecated): fec0::/10
    let first = ip.segments()[0];
    (first & 0xffc0) == 0xfec0
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::State;
    use axum::routing::{get, post};
    use axum::{Json, Router};
    use std::ffi::OsString;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn decode_process_output_decodes_utf16le_like_powershell() {
        // PowerShell commonly emits UTF-16LE when stdout/stderr is piped.
        let mut bytes = Vec::new();
        for b in b"READY\r\n" {
            bytes.push(*b);
            bytes.push(0);
        }

        let out = Toolset::decode_process_output(bytes);
        assert!(out.contains("READY"), "{out:?}");
    }

    #[test]
    fn validate_relative_path_rejects_parent_and_absolute() {
        assert!(validate_relative_path("../a").is_err());
        assert!(validate_relative_path("/etc/passwd").is_err());
        assert!(validate_relative_path("").is_err());
        assert!(validate_relative_path(".").is_err());
        assert!(validate_relative_path("./../a").is_err());
    }

    #[test]
    fn tool_definitions_include_browser_tools() {
        let tmp = tempfile::tempdir().unwrap();
        let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();
        let defs = tools
            .definitions()
            .into_iter()
            .map(|d| d.function.name)
            .collect::<std::collections::BTreeSet<_>>();

        for name in [
            "browser_navigate",
            "browser_click",
            "browser_type",
            "browser_press_key",
            "browser_wait_for",
            "browser_read_page",
            "browser_screenshot",
            "browser_close",
        ] {
            assert!(defs.contains(name), "missing tool definition: {name}");
        }
    }

    #[test]
    fn tool_definitions_include_compat_aliases_and_stubs() {
        let tmp = tempfile::tempdir().unwrap();
        let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();
        let defs = tools
            .definitions()
            .into_iter()
            .map(|d| d.function.name)
            .collect::<std::collections::BTreeSet<_>>();

        for name in [
            "file_read",
            "file_write",
            "file_list",
            "apply_patch",
            "shell_exec",
            "web_search",
            "memory_store",
            "memory_recall",
            "agent_send",
            "task_post",
            "cron_create",
            "process_start",
            "canvas_present",
        ] {
            assert!(defs.contains(name), "missing tool definition: {name}");
        }
    }

    #[tokio::test]
    async fn compat_file_tools_work_via_aliases() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();

        let tools = Toolset::new(workspace.clone()).unwrap();

        tools
            .call("file_write", r#"{ "path": "a.txt", "content": "hello" }"#)
            .await
            .unwrap();

        let content = tools
            .call("file_read", r#"{ "path": "a.txt" }"#)
            .await
            .unwrap();
        assert_eq!(content, "hello");

        std::fs::create_dir_all(workspace.join("dir")).unwrap();
        std::fs::write(workspace.join("dir").join("b.txt"), "world").unwrap();

        let listing = tools.call("file_list", r#"{ "path": "." }"#).await.unwrap();
        assert!(listing.contains("a.txt"), "{listing}");
        assert!(
            listing.contains("dir/") || listing.contains("dir"),
            "{listing}"
        );
    }

    #[tokio::test]
    async fn compat_apply_patch_adds_and_updates_files() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();
        let tools = Toolset::new(workspace.clone()).unwrap();

        let patch = r#"*** Begin Patch
*** Add File: greet.txt
+hi
*** Update File: greet.txt
@@
-hi
+hello
*** End Patch"#;

        let _ = tools
            .call(
                "apply_patch",
                &format!(
                    r#"{{ "patch": {} }}"#,
                    serde_json::to_string(patch).unwrap()
                ),
            )
            .await
            .unwrap();

        let content = std::fs::read_to_string(workspace.join("greet.txt")).unwrap();
        assert_eq!(content.trim_end(), "hello");
    }

    #[tokio::test]
    async fn runtime_tools_are_reported_as_runtime_implemented() {
        let tmp = tempfile::tempdir().unwrap();
        let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();
        let err = tools.call("agent_send", r#"{}"#).await.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("runtime"), "{msg}");
    }

    #[tokio::test]
    async fn media_describe_returns_basic_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();

        std::fs::write(workspace.join("audio.wav"), b"RIFF....WAVEfmt ").unwrap();

        let tools = Toolset::new(workspace).unwrap();
        let out = tools
            .call("media_describe", r#"{ "path": "audio.wav" }"#)
            .await
            .unwrap();

        let v: serde_json::Value = serde_json::from_str(&out).expect("media_describe is json");
        assert_eq!(v.get("path").and_then(|v| v.as_str()), Some("audio.wav"));
        assert_eq!(v.get("bytes").and_then(|v| v.as_u64()), Some(16));
        assert_eq!(v.get("kind").and_then(|v| v.as_str()), Some("audio"));
    }

    #[tokio::test]
    async fn media_transcribe_reads_text_transcripts() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();

        std::fs::write(workspace.join("transcript.txt"), "hello world").unwrap();

        let tools = Toolset::new(workspace).unwrap();
        let out = tools
            .call("media_transcribe", r#"{ "path": "transcript.txt" }"#)
            .await
            .unwrap();

        let v: serde_json::Value =
            serde_json::from_str(&out).expect("media_transcribe output is json");
        assert_eq!(v.get("text").and_then(|v| v.as_str()), Some("hello world"));
    }

    #[tokio::test]
    async fn speech_to_text_reads_text_transcripts() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();

        std::fs::write(workspace.join("transcript.txt"), "hello world").unwrap();

        let tools = Toolset::new(workspace).unwrap();
        let out = tools
            .call("speech_to_text", r#"{ "path": "transcript.txt" }"#)
            .await
            .unwrap();

        let v: serde_json::Value =
            serde_json::from_str(&out).expect("speech_to_text output is json");
        assert_eq!(
            v.get("transcript").and_then(|v| v.as_str()),
            Some("hello world")
        );
        assert_eq!(v.get("text").and_then(|v| v.as_str()), Some("hello world"));
    }

    #[tokio::test]
    async fn text_to_speech_writes_wav_file() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();

        let tools = Toolset::new(workspace.clone()).unwrap();
        let out = tools
            .call(
                "text_to_speech",
                r#"{ "text": "hello", "path": "out.wav" }"#,
            )
            .await
            .unwrap();

        let v: serde_json::Value =
            serde_json::from_str(&out).expect("text_to_speech output is json");
        assert_eq!(v.get("path").and_then(|v| v.as_str()), Some("out.wav"));
        assert_eq!(v.get("format").and_then(|v| v.as_str()), Some("wav"));

        let bytes = std::fs::read(workspace.join("out.wav")).unwrap();
        assert!(bytes.starts_with(b"RIFF"), "missing RIFF header");
        assert!(
            bytes.windows(4).any(|w| w == b"WAVE"),
            "missing WAVE header"
        );
    }

    #[tokio::test]
    async fn image_generate_writes_svg_file() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();

        let tools = Toolset::new(workspace.clone()).unwrap();
        let out = tools
            .call(
                "image_generate",
                r#"{ "prompt": "hello", "path": "out.svg" }"#,
            )
            .await
            .unwrap();

        let v: serde_json::Value = serde_json::from_str(&out).expect("image_generate is json");
        assert_eq!(v.get("path").and_then(|v| v.as_str()), Some("out.svg"));
        assert_eq!(v.get("format").and_then(|v| v.as_str()), Some("svg"));

        let svg = std::fs::read_to_string(workspace.join("out.svg")).unwrap();
        assert!(svg.starts_with("<svg"), "{svg}");
        assert!(svg.contains("hello"), "{svg}");
    }

    #[tokio::test]
    async fn hand_tools_are_reported_as_runtime_implemented() {
        let tmp = tempfile::tempdir().unwrap();
        let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();
        let err = tools.call("hand_list", r#"{}"#).await.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("runtime"), "{msg}");
    }

    #[tokio::test]
    async fn a2a_discover_denies_loopback_by_default() {
        let tmp = tempfile::tempdir().unwrap();
        let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();
        let err = tools
            .call("a2a_discover", r#"{ "url": "http://127.0.0.1:1/" }"#)
            .await
            .unwrap_err();

        let msg = err.to_string();
        assert!(
            msg.contains("loopback") || msg.contains("private") || msg.contains("denied"),
            "{msg}"
        );
    }

    #[tokio::test]
    async fn a2a_discover_fetches_agent_card_when_allow_private_true() {
        async fn handler() -> Json<serde_json::Value> {
            Json(serde_json::json!({
                "name": "demo-agent",
                "description": "demo",
                "url": "http://example.invalid/a2a",
                "version": "1.0",
                "capabilities": { "streaming": false, "pushNotifications": false, "stateTransitionHistory": false },
                "skills": [],
                "defaultInputModes": ["text"],
                "defaultOutputModes": ["text"]
            }))
        }

        let app = Router::new().route("/.well-known/agent.json", get(handler));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let tmp = tempfile::tempdir().unwrap();
        let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();
        let out = tools
            .call(
                "a2a_discover",
                &format!(
                    r#"{{ "url": "http://127.0.0.1:{}/", "allow_private": true }}"#,
                    addr.port()
                ),
            )
            .await
            .unwrap();

        let v: serde_json::Value = serde_json::from_str(&out).expect("a2a_discover output is json");
        assert_eq!(v.get("name").and_then(|v| v.as_str()), Some("demo-agent"));

        server.abort();
    }

    #[derive(Clone, Default)]
    struct A2aSendState {
        last_method: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    }

    #[tokio::test]
    async fn a2a_send_posts_jsonrpc_and_returns_result() {
        async fn handler(
            State(state): State<A2aSendState>,
            Json(payload): Json<serde_json::Value>,
        ) -> Json<serde_json::Value> {
            let method = payload
                .get("method")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            *state.last_method.lock().unwrap() = Some(method.clone());

            if method != "tasks/send" {
                return Json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "error": { "message": "unexpected method" }
                }));
            }

            Json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": {
                    "id": "task-1",
                    "status": "Completed",
                    "messages": [{"role":"agent","parts":[{"type":"text","text":"ok"}]}],
                    "artifacts": []
                }
            }))
        }

        let state = A2aSendState::default();
        let app = Router::new()
            .route("/a2a", post(handler))
            .with_state(state.clone());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let tmp = tempfile::tempdir().unwrap();
        let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();
        let out = tools
            .call(
                "a2a_send",
                &format!(
                    r#"{{ "agent_url": "http://127.0.0.1:{}/a2a", "message": "hello", "allow_private": true }}"#,
                    addr.port()
                ),
            )
            .await
            .unwrap();

        let v: serde_json::Value = serde_json::from_str(&out).expect("a2a_send output is json");
        assert_eq!(v.get("id").and_then(|v| v.as_str()), Some("task-1"), "{v}");
        assert_eq!(
            state.last_method.lock().unwrap().as_deref(),
            Some("tasks/send")
        );

        server.abort();
    }

    #[tokio::test]
    async fn docker_exec_is_disabled_by_default() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let previous = std::env::var_os("REXOS_DOCKER_EXEC_ENABLED");
        std::env::remove_var("REXOS_DOCKER_EXEC_ENABLED");

        let tmp = tempfile::tempdir().unwrap();
        let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();
        let err = tools
            .call("docker_exec", r#"{ "command": "echo hi" }"#)
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("REXOS_DOCKER_EXEC_ENABLED") || msg.contains("disabled"),
            "{msg}"
        );

        match previous {
            Some(v) => std::env::set_var("REXOS_DOCKER_EXEC_ENABLED", v),
            None => std::env::remove_var("REXOS_DOCKER_EXEC_ENABLED"),
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
            arr.iter().any(|p| {
                p.get("process_id").and_then(|v| v.as_str()) == Some(process_id.as_str())
            }),
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
            !arr.iter().any(|p| {
                p.get("process_id").and_then(|v| v.as_str()) == Some(process_id.as_str())
            }),
            "process still listed after kill: {lv}"
        );
    }

    #[tokio::test]
    async fn canvas_present_writes_sanitized_html_file() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();

        let tools = Toolset::new(workspace.clone()).unwrap();
        let out = tools
            .call(
                "canvas_present",
                r#"{ "title": "Report", "html": "<h1>Hello</h1>" }"#,
            )
            .await
            .unwrap();

        let v: serde_json::Value =
            serde_json::from_str(&out).expect("canvas_present output is json");
        let saved_to = v
            .get("saved_to")
            .and_then(|v| v.as_str())
            .expect("saved_to");
        assert!(
            saved_to.ends_with(".html"),
            "unexpected saved_to: {saved_to}"
        );

        let html = std::fs::read_to_string(workspace.join(saved_to)).unwrap();
        assert!(html.contains("<h1>Hello</h1>"), "{html}");
        assert!(html.contains("<title>Report</title>"), "{html}");
    }

    #[tokio::test]
    async fn canvas_present_rejects_dangerous_html() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();

        let tools = Toolset::new(workspace).unwrap();

        let err = tools
            .call(
                "canvas_present",
                r#"{ "html": "<script>alert(1)</script>" }"#,
            )
            .await
            .unwrap_err();
        assert!(err.to_string().to_lowercase().contains("script"), "{err}");

        let err = tools
            .call(
                "canvas_present",
                r#"{ "html": "<img src=x onerror=alert(1)>" }"#,
            )
            .await
            .unwrap_err();
        assert!(
            err.to_string().to_lowercase().contains("event")
                || err.to_string().to_lowercase().contains("onerror")
                || err.to_string().to_lowercase().contains("handler"),
            "{err}"
        );

        let err = tools
            .call(
                "canvas_present",
                r#"{ "html": "<a href=\"javascript:alert(1)\">x</a>" }"#,
            )
            .await
            .unwrap_err();
        assert!(
            err.to_string().to_lowercase().contains("javascript"),
            "{err}"
        );
    }

    #[tokio::test]
    async fn image_analyze_returns_dimensions_for_png() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();

        let png_1x1 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+X2OQAAAAASUVORK5CYII=";
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(png_1x1)
            .expect("decode png base64");
        std::fs::write(workspace.join("img.png"), bytes).unwrap();

        let tools = Toolset::new(workspace).unwrap();
        let out = tools
            .call("image_analyze", r#"{ "path": "img.png" }"#)
            .await
            .unwrap();

        let v: serde_json::Value = serde_json::from_str(&out).expect("image_analyze is json");
        assert_eq!(v.get("width").and_then(|v| v.as_u64()), Some(1), "{v}");
        assert_eq!(v.get("height").and_then(|v| v.as_u64()), Some(1), "{v}");
    }

    #[tokio::test]
    async fn location_get_returns_environment_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();
        let out = tools.call("location_get", r#"{}"#).await.unwrap();

        let v: serde_json::Value = serde_json::from_str(&out).expect("location_get is json");
        assert_eq!(
            v.get("os").and_then(|v| v.as_str()),
            Some(std::env::consts::OS),
            "{v}"
        );
        assert_eq!(
            v.get("arch").and_then(|v| v.as_str()),
            Some(std::env::consts::ARCH),
            "{v}"
        );
    }

    #[tokio::test]
    async fn browser_navigate_denies_loopback_by_default() {
        let tmp = tempfile::tempdir().unwrap();
        let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();
        let err = tools
            .call(
                "browser_navigate",
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
    async fn browser_navigate_allows_loopback_when_allow_private_true() {
        let tmp = tempfile::tempdir().unwrap();
        let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();
        let res = tools
            .call(
                "browser_navigate",
                r#"{ "url": "http://127.0.0.1:1/", "allow_private": true }"#,
            )
            .await;

        match res {
            Ok(out) => assert!(!out.trim().is_empty()),
            Err(err) => {
                let msg = err.to_string();
                assert!(
                    !msg.contains("loopback/private address"),
                    "unexpected SSRF-style error: {msg}"
                );
            }
        }
    }

    #[tokio::test]
    async fn browser_close_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();

        let out = tools.call("browser_close", r#"{}"#).await.unwrap();
        assert_eq!(out.trim(), "ok");
    }

    #[tokio::test]
    async fn browser_click_requires_session() {
        let tmp = tempfile::tempdir().unwrap();
        let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();

        let err = tools
            .call("browser_click", r#"{ "selector": "a" }"#)
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("browser_navigate") || msg.contains("session"),
            "{msg}"
        );
    }

    #[tokio::test]
    async fn browser_press_key_requires_session() {
        let tmp = tempfile::tempdir().unwrap();
        let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();

        let err = tools
            .call("browser_press_key", r#"{ "key": "Enter" }"#)
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("browser_navigate") || msg.contains("session"),
            "{msg}"
        );
    }

    #[tokio::test]
    async fn browser_wait_for_requires_session() {
        let tmp = tempfile::tempdir().unwrap();
        let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();

        let err = tools
            .call("browser_wait_for", r#"{ "text": "hello" }"#)
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("browser_navigate") || msg.contains("session"),
            "{msg}"
        );
    }

    #[tokio::test]
    async fn browser_read_page_requires_session() {
        let tmp = tempfile::tempdir().unwrap();
        let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();

        let err = tools.call("browser_read_page", r#"{}"#).await.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("browser_navigate") || msg.contains("session"),
            "{msg}"
        );
    }

    #[tokio::test]
    async fn browser_screenshot_requires_session() {
        let tmp = tempfile::tempdir().unwrap();
        let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();

        let err = tools
            .call("browser_screenshot", r#"{ "path": "shot.png" }"#)
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("browser_navigate") || msg.contains("session"),
            "{msg}"
        );
    }

    #[tokio::test]
    async fn browser_tools_work_with_stub_bridge() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();

        let bridge_path = tmp.path().join("bridge.py");
        std::fs::write(&bridge_path, stub_bridge_script()).unwrap();

        let python = if cfg!(windows) { "python" } else { "python3" };
        let _backend_guard = EnvVarGuard::set("REXOS_BROWSER_BACKEND", "playwright");
        let _python_guard = EnvVarGuard::set("REXOS_BROWSER_PYTHON", python);
        let _bridge_guard = EnvVarGuard::set("REXOS_BROWSER_BRIDGE_PATH", bridge_path.as_os_str());

        let tools = Toolset::new(workspace.clone()).unwrap();

        let _ = tools
            .call(
                "browser_navigate",
                r#"{ "url": "http://127.0.0.1:1/", "allow_private": true }"#,
            )
            .await
            .unwrap();

        let out = tools
            .call("browser_press_key", r#"{ "key": "Enter" }"#)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["key"], "Enter");

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

        let out = tools.call("browser_close", r#"{}"#).await.unwrap();
        assert_eq!(out.trim(), "ok");
    }

    #[tokio::test]
    async fn browser_navigate_honors_headless_flag() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();

        let bridge_path = tmp.path().join("bridge.py");
        std::fs::write(&bridge_path, stub_bridge_script()).unwrap();

        let python = if cfg!(windows) { "python" } else { "python3" };
        let _backend_guard = EnvVarGuard::set("REXOS_BROWSER_BACKEND", "playwright");
        let _python_guard = EnvVarGuard::set("REXOS_BROWSER_PYTHON", python);
        let _bridge_guard = EnvVarGuard::set("REXOS_BROWSER_BRIDGE_PATH", bridge_path.as_os_str());

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

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<OsString>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
            let previous = std::env::var_os(key);
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(v) => std::env::set_var(self.key, v),
                None => std::env::remove_var(self.key),
            }
        }
    }

    fn stub_bridge_script() -> &'static str {
        r#"import argparse
import json
import sys

PNG_B64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMB/6X9Yt8AAAAASUVORK5CYII="

parser = argparse.ArgumentParser()
parser.add_argument("--headless", action="store_true", default=True)
parser.add_argument("--no-headless", dest="headless", action="store_false")
parser.add_argument("--width", type=int, default=1280)
parser.add_argument("--height", type=int, default=720)
parser.add_argument("--timeout", type=int, default=30)
args = parser.parse_args()
headless = bool(args.headless)

sys.stdout.write(json.dumps({"success": True, "data": {"status": "ready"}}) + "\n")
sys.stdout.flush()

current_url = ""

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    cmd = json.loads(line)
    action = cmd.get("action", "")
    if action == "Navigate":
        current_url = cmd.get("url", "")
        resp = {"success": True, "data": {"title": "Stub", "url": current_url, "headless": headless}}
    elif action == "ReadPage":
        resp = {"success": True, "data": {"title": "Stub", "url": current_url, "content": "hello"}}
    elif action == "Screenshot":
        resp = {"success": True, "data": {"format": "png", "url": current_url, "image_base64": PNG_B64}}
    elif action == "Click":
        resp = {"success": True, "data": {"clicked": cmd.get("selector", "")}}
    elif action == "Type":
        resp = {"success": True, "data": {"typed": cmd.get("text", ""), "selector": cmd.get("selector", "")}}
    elif action == "PressKey":
        resp = {"success": True, "data": {"key": cmd.get("key", ""), "selector": cmd.get("selector", "")}}
    elif action == "WaitFor":
        waited_for = {}
        if cmd.get("selector"):
            waited_for["selector"] = cmd.get("selector", "")
        if cmd.get("text"):
            waited_for["text"] = cmd.get("text", "")
        resp = {"success": True, "data": {"waited_for": waited_for, "timeout_ms": cmd.get("timeout_ms")}}
    elif action == "Close":
        resp = {"success": True, "data": {"status": "closed"}}
        sys.stdout.write(json.dumps(resp) + "\n")
        sys.stdout.flush()
        break
    else:
        resp = {"success": False, "error": "unknown action"}

    sys.stdout.write(json.dumps(resp) + "\n")
    sys.stdout.flush()
"#
    }
}
