use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use anyhow::{bail, Context};
use base64::Engine as _;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use rexos_llm::openai_compat::{ToolDefinition, ToolFunctionDefinition};

const BROWSER_BRIDGE_SCRIPT: &str = include_str!("browser_bridge.py");
static BROWSER_BRIDGE_PATH: OnceLock<PathBuf> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct Toolset {
    workspace_root: PathBuf,
    http: reqwest::Client,
    browser: Arc<tokio::sync::Mutex<Option<BrowserSession>>>,
}

impl Toolset {
    pub fn new(workspace_root: PathBuf) -> anyhow::Result<Self> {
        let workspace_root = workspace_root
            .canonicalize()
            .with_context(|| format!("canonicalize workspace root: {}", workspace_root.display()))?;
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .build()
            .context("build http client")?;
        Ok(Self {
            workspace_root,
            http,
            browser: Arc::new(tokio::sync::Mutex::new(None)),
        })
    }

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        vec![
            fs_read_def(),
            fs_write_def(),
            shell_def(),
            web_fetch_def(),
            browser_navigate_def(),
            browser_click_def(),
            browser_type_def(),
            browser_read_page_def(),
            browser_screenshot_def(),
            browser_close_def(),
        ]
    }

    pub async fn call(&self, name: &str, arguments_json: &str) -> anyhow::Result<String> {
        match name {
            "fs_read" => {
                let args: FsReadArgs = serde_json::from_str(arguments_json)
                    .context("parse fs_read arguments")?;
                self.fs_read(&args.path)
            }
            "fs_write" => {
                let args: FsWriteArgs = serde_json::from_str(arguments_json)
                    .context("parse fs_write arguments")?;
                self.fs_write(&args.path, &args.content)
            }
            "shell" => {
                let args: ShellArgs = serde_json::from_str(arguments_json)
                    .context("parse shell arguments")?;
                self.shell(&args.command, args.timeout_ms).await
            }
            "web_fetch" => {
                let args: WebFetchArgs = serde_json::from_str(arguments_json)
                    .context("parse web_fetch arguments")?;
                self.web_fetch(
                    &args.url,
                    args.timeout_ms,
                    args.max_bytes,
                    args.allow_private,
                )
                .await
            }
            "browser_navigate" => {
                let args: BrowserNavigateArgs = serde_json::from_str(arguments_json)
                    .context("parse browser_navigate arguments")?;
                self.browser_navigate(&args.url, args.timeout_ms, args.allow_private)
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
                let args: BrowserTypeArgs = serde_json::from_str(arguments_json)
                    .context("parse browser_type arguments")?;
                self.browser_type(&args.selector, &args.text).await
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
            _ => bail!("unknown tool: {name}"),
        }
    }

    fn fs_read(&self, user_path: &str) -> anyhow::Result<String> {
        let path = self.resolve_workspace_path(user_path)?;

        let meta = std::fs::metadata(&path)
            .with_context(|| format!("stat {}", path.display()))?;
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
        let port = url
            .port_or_known_default()
            .context("url missing port")?;

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
        let slice = if truncated { &bytes[..max_bytes] } else { &bytes };
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

    async fn browser_navigate(
        &self,
        url: &str,
        _timeout_ms: Option<u64>,
        allow_private: bool,
    ) -> anyhow::Result<String> {
        let url = reqwest::Url::parse(url).context("parse url")?;
        match url.scheme() {
            "http" | "https" => {}
            _ => bail!("only http/https urls are allowed"),
        }

        let host = url.host_str().context("url missing host")?;
        let port = url
            .port_or_known_default()
            .context("url missing port")?;

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

        let mut guard = self.browser.lock().await;
        if guard.is_none() {
            *guard = Some(BrowserSession::spawn().await?);
        }

        let session = guard.as_mut().expect("set above");
        let resp = session
            .send(serde_json::json!({
                "action": "Navigate",
                "url": url.as_str(),
            }))
            .await?;

        Ok(resp.into_tool_output()?)
    }

    async fn browser_close(&self) -> anyhow::Result<String> {
        let mut guard = self.browser.lock().await;
        if let Some(mut session) = guard.take() {
            let _ = session
                .send(serde_json::json!({ "action": "Close" }))
                .await;
            session.kill().await;
        }
        Ok("ok".to_string())
    }

    async fn browser_click(&self, selector: &str) -> anyhow::Result<String> {
        let mut guard = self.browser.lock().await;
        let session = guard
            .as_mut()
            .context("browser session not started; call browser_navigate first")?;
        let resp = session
            .send(serde_json::json!({
                "action": "Click",
                "selector": selector,
            }))
            .await?;
        Ok(resp.into_tool_output()?)
    }

    async fn browser_type(&self, selector: &str, text: &str) -> anyhow::Result<String> {
        let mut guard = self.browser.lock().await;
        let session = guard
            .as_mut()
            .context("browser session not started; call browser_navigate first")?;
        let resp = session
            .send(serde_json::json!({
                "action": "Type",
                "selector": selector,
                "text": text,
            }))
            .await?;
        Ok(resp.into_tool_output()?)
    }

    async fn browser_read_page(&self) -> anyhow::Result<String> {
        let mut guard = self.browser.lock().await;
        let session = guard
            .as_mut()
            .context("browser session not started; call browser_navigate first")?;
        let resp = session.send(serde_json::json!({ "action": "ReadPage" })).await?;
        Ok(resp.into_tool_output()?)
    }

    async fn browser_screenshot(&self, path: Option<&str>) -> anyhow::Result<String> {
        let mut guard = self.browser.lock().await;
        let session = guard
            .as_mut()
            .context("browser session not started; call browser_navigate first")?;

        let resp = session
            .send(serde_json::json!({ "action": "Screenshot" }))
            .await?;
        let data = resp.into_data()?;

        let b64 = data
            .get("image_base64")
            .and_then(|v| v.as_str())
            .context("bridge response missing image_base64")?;
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

struct BrowserSession {
    child: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
}

impl std::fmt::Debug for BrowserSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserSession").finish_non_exhaustive()
    }
}

impl BrowserSession {
    async fn spawn() -> anyhow::Result<Self> {
        let python = browser_python_exe();
        let script_path = browser_bridge_script_path()?;

        let mut cmd = tokio::process::Command::new(python);
        cmd.arg("-u").arg(script_path);
        cmd.args(["--headless", "--width", "1280", "--height", "720", "--timeout", "30"]);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        sandbox_python_env(&mut cmd);

        let mut child = cmd.spawn().context("spawn browser bridge")?;
        let stdin = child.stdin.take().context("capture bridge stdin")?;
        let stdout = child.stdout.take().context("capture bridge stdout")?;

        let mut session = Self {
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

impl Drop for BrowserSession {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
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
struct FsWriteArgs {
    path: String,
    content: String,
}

#[derive(Debug, serde::Deserialize)]
struct ShellArgs {
    command: String,
    #[serde(default)]
    timeout_ms: Option<u64>,
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
struct BrowserNavigateArgs {
    url: String,
    #[serde(default)]
    timeout_ms: Option<u64>,
    #[serde(default)]
    allow_private: bool,
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

fn fs_write_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "fs_write".to_string(),
            description: "Write a UTF-8 text file to the workspace (creates parent dirs).".to_string(),
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
            description: "Run a shell command inside the workspace (bash on Unix, PowerShell on Windows).".to_string(),
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
            description: "Fetch a URL via HTTP(S) and return a small response body (SSRF-protected).".to_string(),
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
                    "allow_private": { "type": "boolean", "description": "Allow loopback/private IPs (default false)." }
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
            description: "Click an element in the browser by CSS selector (or best-effort text fallback).".to_string(),
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
    use std::ffi::OsString;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

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
            "browser_read_page",
            "browser_screenshot",
            "browser_close",
        ] {
            assert!(defs.contains(name), "missing tool definition: {name}");
        }
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
        let _lock = ENV_LOCK.lock().unwrap();

        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();

        let bridge_path = tmp.path().join("bridge.py");
        std::fs::write(&bridge_path, stub_bridge_script()).unwrap();

        let python = if cfg!(windows) { "python" } else { "python3" };
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
parser.parse_args()

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
        resp = {"success": True, "data": {"title": "Stub", "url": current_url}}
    elif action == "ReadPage":
        resp = {"success": True, "data": {"title": "Stub", "url": current_url, "content": "hello"}}
    elif action == "Screenshot":
        resp = {"success": True, "data": {"format": "png", "url": current_url, "image_base64": PNG_B64}}
    elif action == "Click":
        resp = {"success": True, "data": {"clicked": cmd.get("selector", "")}}
    elif action == "Type":
        resp = {"success": True, "data": {"typed": cmd.get("text", ""), "selector": cmd.get("selector", "")}}
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
