use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use anyhow::{bail, Context};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use rexos_llm::openai_compat::ToolDefinition;

const BROWSER_BRIDGE_SCRIPT: &str = include_str!("browser_bridge.py");
static BROWSER_BRIDGE_PATH: OnceLock<PathBuf> = OnceLock::new();

mod browser_cdp;
mod defs;
mod dispatch;
mod ops;
mod patch;

use defs::{compat_tool_defs, core_tool_defs};

#[derive(Debug, Clone)]
pub struct Toolset {
    workspace_root: PathBuf,
    http: reqwest::Client,
    browser: Arc<tokio::sync::Mutex<Option<BrowserSession>>>,
    processes: Arc<tokio::sync::Mutex<ProcessManager>>,
    allowed_tools: Option<HashSet<String>>,
}

const PROCESS_MAX_PROCESSES: usize = 5;
const PROCESS_OUTPUT_MAX_BYTES: usize = 200_000;
const PROCESS_OUTPUT_HEAD_MAX_BYTES: usize = 20_000;
const PROCESS_OUTPUT_TAIL_MAX_BYTES: usize =
    PROCESS_OUTPUT_MAX_BYTES - PROCESS_OUTPUT_HEAD_MAX_BYTES;
const TOOL_OUTPUT_MIDDLE_OMISSION_MARKER: &str = "\n\n[... middle omitted ...]\n\n";

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

#[derive(Debug, Default)]
struct ProcessOutputBuffer {
    head: Vec<u8>,
    tail: Vec<u8>,
    total_bytes: usize,
}

impl ProcessOutputBuffer {
    fn push(&mut self, chunk: &[u8]) {
        if chunk.is_empty() {
            return;
        }

        self.total_bytes = self.total_bytes.saturating_add(chunk.len());

        if self.head.len() < PROCESS_OUTPUT_HEAD_MAX_BYTES {
            let remaining = PROCESS_OUTPUT_HEAD_MAX_BYTES - self.head.len();
            let take = remaining.min(chunk.len());
            self.head.extend_from_slice(&chunk[..take]);
        }

        self.tail.extend_from_slice(chunk);
        if self.tail.len() > PROCESS_OUTPUT_TAIL_MAX_BYTES {
            let start = self.tail.len() - PROCESS_OUTPUT_TAIL_MAX_BYTES;
            let tail = self.tail.split_off(start);
            self.tail = tail;
        }
    }

    fn take_text(&mut self) -> (String, bool) {
        if self.total_bytes == 0 {
            return (String::new(), false);
        }

        let truncated = self.total_bytes > PROCESS_OUTPUT_MAX_BYTES;
        let out = if truncated {
            let head = Toolset::decode_process_output(self.head.clone());
            let tail = Toolset::decode_process_output(self.tail.clone());
            format!("{head}{TOOL_OUTPUT_MIDDLE_OMISSION_MARKER}{tail}")
        } else {
            let bytes = self.reconstruct_all_bytes();
            Toolset::decode_process_output(bytes)
        };

        self.head.clear();
        self.tail.clear();
        self.total_bytes = 0;

        (out, truncated)
    }

    fn reconstruct_all_bytes(&self) -> Vec<u8> {
        if self.total_bytes <= self.tail.len() {
            return self.tail.clone();
        }

        let tail_start = self.total_bytes.saturating_sub(self.tail.len());
        let overlap = self.head.len().saturating_sub(tail_start);

        let mut out = Vec::with_capacity(self.head.len() + self.tail.len().saturating_sub(overlap));
        out.extend_from_slice(&self.head);
        if overlap < self.tail.len() {
            out.extend_from_slice(&self.tail[overlap..]);
        }
        out
    }
}

struct ProcessEntry {
    command: String,
    args: Vec<String>,
    started_at: std::time::Instant,
    exit_code: Option<i32>,
    child: tokio::process::Child,
    stdin: Option<tokio::process::ChildStdin>,
    stdout: Arc<tokio::sync::Mutex<ProcessOutputBuffer>>,
    stderr: Arc<tokio::sync::Mutex<ProcessOutputBuffer>>,
}

impl Drop for ProcessEntry {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

impl Toolset {
    pub fn new(workspace_root: PathBuf) -> anyhow::Result<Self> {
        Self::new_with_allowed_tools(workspace_root, None)
    }

    pub fn new_with_allowed_tools(
        workspace_root: PathBuf,
        allowed_tools: Option<Vec<String>>,
    ) -> anyhow::Result<Self> {
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
            allowed_tools: allowed_tools.map(|tools| {
                tools
                    .into_iter()
                    .map(|name| name.trim().to_string())
                    .filter(|name| !name.is_empty())
                    .collect()
            }),
        })
    }

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        let mut defs = core_tool_defs();
        defs.extend(compat_tool_defs());
        if let Some(allowed) = self.allowed_tools.as_ref() {
            defs.retain(|def| allowed.contains(def.function.name.as_str()));
        }
        defs
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BrowserBackend {
    Cdp,
    Playwright,
}

fn browser_backend_default() -> BrowserBackend {
    if let Ok(v) = std::env::var("LOOPFORGE_BROWSER_BACKEND") {
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

    async fn back(&mut self) -> anyhow::Result<serde_json::Value> {
        match self {
            Self::Cdp(s) => s.back().await,
            Self::Playwright(s) => Ok(s
                .send(serde_json::json!({ "action": "Back" }))
                .await?
                .into_data()?),
        }
    }

    async fn scroll(&mut self, direction: &str, amount: i64) -> anyhow::Result<serde_json::Value> {
        match self {
            Self::Cdp(s) => s.scroll(direction, amount).await,
            Self::Playwright(s) => Ok(s
                .send(serde_json::json!({
                    "action": "Scroll",
                    "direction": direction,
                    "amount": amount,
                }))
                .await?
                .into_data()?),
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

    async fn run_js(&mut self, expression: &str) -> anyhow::Result<serde_json::Value> {
        match self {
            Self::Cdp(s) => {
                let result = s.run_js(expression).await?;
                let url = s.current_url().await.ok();
                Ok(serde_json::json!({
                    "result": result,
                    "url": url,
                }))
            }
            Self::Playwright(s) => Ok(s
                .send(serde_json::json!({
                    "action": "RunJs",
                    "expression": expression,
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
    if let Ok(v) = std::env::var("LOOPFORGE_BROWSER_HEADLESS") {
        match v.trim().to_ascii_lowercase().as_str() {
            "0" | "false" | "no" | "off" => return false,
            "1" | "true" | "yes" | "on" => return true,
            _ => {}
        }
    }
    true
}

fn browser_python_exe() -> String {
    if let Ok(v) = std::env::var("LOOPFORGE_BROWSER_PYTHON") {
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
    if let Ok(v) = std::env::var("LOOPFORGE_BROWSER_BRIDGE_PATH") {
        let p = PathBuf::from(v);
        if p.exists() {
            return Ok(p);
        }
        bail!(
            "LOOPFORGE_BROWSER_BRIDGE_PATH does not exist: {}",
            p.display()
        );
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

fn extract_between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let start_idx = text.find(start)? + start.len();
    let remaining = &text[start_idx..];
    let end_idx = remaining.find(end)?;
    Some(&remaining[..end_idx])
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
    use crate::defs::ensure_browser_url_allowed;
    use crate::ops::fs::validate_relative_path;
    use axum::extract::State;
    use axum::routing::{get, post};
    use axum::{Json, Router};
    use base64::Engine as _;
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

    #[tokio::test]
    async fn ensure_browser_url_allowed_rejects_file_scheme_even_when_allow_private_true() {
        let err = ensure_browser_url_allowed("file:///etc/passwd", true)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("http/https"), "{err}");
    }

    #[tokio::test]
    async fn ensure_browser_url_allowed_allows_about_blank() {
        ensure_browser_url_allowed("about:blank", false)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn ensure_browser_url_allowed_allows_chrome_error_page() {
        ensure_browser_url_allowed("chrome-error://chromewebdata/", false)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn ensure_browser_url_allowed_allows_public_ip_http() {
        ensure_browser_url_allowed("http://1.1.1.1", false)
            .await
            .unwrap();
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
            "browser_back",
            "browser_scroll",
            "browser_click",
            "browser_type",
            "browser_press_key",
            "browser_wait",
            "browser_wait_for",
            "browser_read_page",
            "browser_run_js",
            "browser_screenshot",
            "browser_close",
        ] {
            assert!(defs.contains(name), "missing tool definition: {name}");
        }
    }

    #[test]
    fn tool_definitions_include_pdf() {
        let tmp = tempfile::tempdir().unwrap();
        let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();
        let defs = tools
            .definitions()
            .into_iter()
            .map(|d| d.function.name)
            .collect::<std::collections::BTreeSet<_>>();

        for name in ["pdf", "pdf_extract"] {
            assert!(defs.contains(name), "missing tool definition: {name}");
        }
    }

    #[test]
    fn browser_bridge_script_includes_back_scroll_and_run_js_actions() {
        // The built-in Playwright bridge script should support the same browser tool surface.
        for needle in ["\"Back\"", "\"Scroll\"", "\"RunJs\""] {
            assert!(
                super::BROWSER_BRIDGE_SCRIPT.contains(needle),
                "bridge script missing action handler: {needle}"
            );
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

    #[test]
    fn tool_call_domain_classifies_core_and_compat_tools() {
        use super::dispatch::{tool_call_domain, ToolCallDomain};

        assert_eq!(tool_call_domain("fs_read"), Some(ToolCallDomain::Fs));
        assert_eq!(
            tool_call_domain("shell_exec"),
            Some(ToolCallDomain::Process)
        );
        assert_eq!(tool_call_domain("pdf_extract"), Some(ToolCallDomain::Web));
        assert_eq!(tool_call_domain("location_get"), Some(ToolCallDomain::Web));
        assert_eq!(
            tool_call_domain("image_generate"),
            Some(ToolCallDomain::Media)
        );
        assert_eq!(
            tool_call_domain("browser_run_js"),
            Some(ToolCallDomain::Browser)
        );
        assert_eq!(
            tool_call_domain("workflow_run"),
            Some(ToolCallDomain::RuntimeCompat)
        );
        assert_eq!(tool_call_domain("unknown_tool"), None);
    }

    #[tokio::test]
    async fn pdf_extracts_text_from_fixture() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();

        let pdf_b64 = "JVBERi0xLjQKMSAwIG9iago8PCAvVHlwZSAvQ2F0YWxvZyAvUGFnZXMgMiAwIFIgPj4KZW5kb2JqCjIgMCBvYmoKPDwgL1R5cGUgL1BhZ2VzIC9LaWRzIFszIDAgUl0gL0NvdW50IDEgPj4KZW5kb2JqCjMgMCBvYmoKPDwgL1R5cGUgL1BhZ2UgL1BhcmVudCAyIDAgUiAvTWVkaWFCb3ggWzAgMCA2MTIgNzkyXSAvQ29udGVudHMgNCAwIFIgL1Jlc291cmNlcyA8PCAvRm9udCA8PCAvRjEgNSAwIFIgPj4gPj4gPj4KZW5kb2JqCjQgMCBvYmoKPDwgL0xlbmd0aCA0OCA+PgpzdHJlYW0KQlQKL0YxIDI0IFRmCjEwMCA3MDAgVGQKKEhlbGxvIFJleE9TIFBERikgVGoKRVQKZW5kc3RyZWFtCmVuZG9iago1IDAgb2JqCjw8IC9UeXBlIC9Gb250IC9TdWJ0eXBlIC9UeXBlMSAvQmFzZUZvbnQgL0hlbHZldGljYSA+PgplbmRvYmoKeHJlZgowIDYKMDAwMDAwMDAwMCA2NTUzNSBmIAowMDAwMDAwMDA5IDAwMDAwIG4gCjAwMDAwMDAwNTggMDAwMDAgbiAKMDAwMDAwMDExNSAwMDAwMCBuIAowMDAwMDAwMjQxIDAwMDAwIG4gCjAwMDAwMDAzMzggMDAwMDAgbiAKdHJhaWxlcgo8PCAvU2l6ZSA2IC9Sb290IDEgMCBSID4+CnN0YXJ0eHJlZgo0MDgKJSVFT0YK";
        let pdf_bytes = base64::engine::general_purpose::STANDARD
            .decode(pdf_b64)
            .unwrap();
        std::fs::write(workspace.join("fixture.pdf"), pdf_bytes).unwrap();

        let tools = Toolset::new(workspace).unwrap();
        let out = tools
            .call("pdf", r#"{ "path": "fixture.pdf" }"#)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let text = v.get("text").and_then(|v| v.as_str()).unwrap_or("");
        assert!(text.contains("Hello") && text.contains("PDF"), "{text}");
        assert_eq!(v.get("path").and_then(|v| v.as_str()), Some("fixture.pdf"));
    }

    #[tokio::test]
    async fn pdf_pages_range_selects_requested_pages() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();

        let pdf_b64 = "JVBERi0xLjQKMSAwIG9iago8PCAvVHlwZSAvQ2F0YWxvZyAvUGFnZXMgMiAwIFIgPj4KZW5kb2JqCjIgMCBvYmoKPDwgL1R5cGUgL1BhZ2VzIC9LaWRzIFszIDAgUiA0IDAgUiA1IDAgUl0gL0NvdW50IDMgPj4KZW5kb2JqCjMgMCBvYmoKPDwgL1R5cGUgL1BhZ2UgL1BhcmVudCAyIDAgUiAvTWVkaWFCb3ggWzAgMCA2MTIgNzkyXSAvQ29udGVudHMgNiAwIFIgL1Jlc291cmNlcyA8PCAvRm9udCA8PCAvRjEgOSAwIFIgPj4gPj4gPj4KZW5kb2JqCjQgMCBvYmoKPDwgL1R5cGUgL1BhZ2UgL1BhcmVudCAyIDAgUiAvTWVkaWFCb3ggWzAgMCA2MTIgNzkyXSAvQ29udGVudHMgNyAwIFIgL1Jlc291cmNlcyA8PCAvRm9udCA8PCAvRjEgOSAwIFIgPj4gPj4gPj4KZW5kb2JqCjUgMCBvYmoKPDwgL1R5cGUgL1BhZ2UgL1BhcmVudCAyIDAgUiAvTWVkaWFCb3ggWzAgMCA2MTIgNzkyXSAvQ29udGVudHMgOCAwIFIgL1Jlc291cmNlcyA8PCAvRm9udCA8PCAvRjEgOSAwIFIgPj4gPj4gPj4KZW5kb2JqCjYgMCBvYmoKPDwgL0xlbmd0aCA0MSA+PgpzdHJlYW0KQlQKL0YxIDI0IFRmCjEwMCA3MDAgVGQKKFBBR0VfT05FKSBUagpFVAplbmRzdHJlYW0KZW5kb2JqCjcgMCBvYmoKPDwgL0xlbmd0aCA0MSA+PgpzdHJlYW0KQlQKL0YxIDI0IFRmCjEwMCA3MDAgVGQKKFBBR0VfVFdPKSBUagpFVAplbmRzdHJlYW0KZW5kb2JqCjggMCBvYmoKPDwgL0xlbmd0aCA0MyA+PgpzdHJlYW0KQlQKL0YxIDI0IFRmCjEwMCA3MDAgVGQKKFBBR0VfVEhSRUUpIFRqCkVUCmVuZHN0cmVhbQplbmRvYmoKOSAwIG9iago8PCAvVHlwZSAvRm9udCAvU3VidHlwZSAvVHlwZTEgL0Jhc2VGb250IC9IZWx2ZXRpY2EgPj4KZW5kb2JqCnhyZWYKMCAxMAowMDAwMDAwMDAwIDY1NTM1IGYgCjAwMDAwMDAwMDkgMDAwMDAgbiAKMDAwMDAwMDA1OCAwMDAwMCBuIAowMDAwMDAwMTI3IDAwMDAwIG4gCjAwMDAwMDAyNTMgMDAwMDAgbiAKMDAwMDAwMDM3OSAwMDAwMCBuIAowMDAwMDAwNTA1IDAwMDAwIG4gCjAwMDAwMDA1OTUgMDAwMDAgbiAKMDAwMDAwMDY4NSAwMDAwMCBuIAowMDAwMDAwNzc3IDAwMDAwIG4gCnRyYWlsZXIKPDwgL1NpemUgMTAgL1Jvb3QgMSAwIFIgPj4Kc3RhcnR4cmVmCjg0NwolJUVPRgo=";
        let pdf_bytes = base64::engine::general_purpose::STANDARD
            .decode(pdf_b64)
            .unwrap();
        std::fs::write(workspace.join("pages.pdf"), pdf_bytes).unwrap();

        let tools = Toolset::new(workspace).unwrap();
        let out = tools
            .call("pdf", r#"{ "path": "pages.pdf", "pages": "2" }"#)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let text = v.get("text").and_then(|v| v.as_str()).unwrap_or("");

        assert!(text.contains("PAGE_TWO"), "{text}");
        assert!(!text.contains("PAGE_ONE"), "{text}");
        assert!(!text.contains("PAGE_THREE"), "{text}");
    }

    #[tokio::test]
    async fn core_file_tools_work_via_primary_names() {
        let tmp = tempfile::tempdir().unwrap();
        let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();

        tools
            .call(
                "fs_write",
                r#"{ "path": "nested/a.txt", "content": "hello core" }"#,
            )
            .await
            .unwrap();

        let out = tools
            .call("fs_read", r#"{ "path": "nested/a.txt" }"#)
            .await
            .unwrap();
        assert_eq!(out, "hello core");

        let listing = tools
            .call("file_list", r#"{ "path": "nested" }"#)
            .await
            .unwrap();
        assert_eq!(listing, "a.txt");
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
    async fn workflow_run_is_reported_as_runtime_implemented() {
        let tmp = tempfile::tempdir().unwrap();
        let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();
        let err = tools
            .call("workflow_run", r#"{ "steps": [] }"#)
            .await
            .unwrap_err();
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
    async fn process_poll_truncation_preserves_head_and_tail() {
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
                    "[Console]::Out.WriteLine('HEAD_START'); [Console]::Out.Write(('A' * 350000)); [Console]::Out.WriteLine(''); [Console]::Out.WriteLine('TAIL_END'); [Console]::Out.Flush(); Start-Sleep -Seconds 5"
                ]
            })
        } else {
            serde_json::json!({
                "command": "bash",
                "args": ["-lc", "echo HEAD_START; head -c 350000 < /dev/zero | tr '\\0' 'A'; echo; echo TAIL_END; sleep 5"]
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

        // Give the process time to emit enough output to overflow the buffer before we poll.
        tokio::time::sleep(Duration::from_millis(if cfg!(windows) {
            1200
        } else {
            500
        }))
        .await;

        let deadline = tokio::time::Instant::now()
            + if cfg!(windows) {
                Duration::from_secs(10)
            } else {
                Duration::from_secs(5)
            };

        let mut seen = String::new();
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
            seen.push_str(stdout);
            seen.push_str(stderr);

            if seen.contains("TAIL_END") {
                break;
            }
            if pv.get("alive").and_then(|v| v.as_bool()) == Some(false) {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        assert!(
            seen.contains("TAIL_END"),
            "did not see TAIL_END\noutput:\n{}",
            seen
        );
        assert!(
            seen.contains("HEAD_START"),
            "expected truncated output to preserve head and tail\noutput:\n{}",
            seen
        );
        assert!(
            seen.contains("[... middle omitted ...]"),
            "expected omission marker in truncated output\noutput:\n{}",
            seen
        );

        let _ = tools
            .call(
                "process_kill",
                &format!(r#"{{ "process_id": "{}" }}"#, process_id),
            )
            .await
            .unwrap();
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
        let _backend_guard = EnvVarGuard::set("LOOPFORGE_BROWSER_BACKEND", "playwright");
        let _python_guard = EnvVarGuard::set("LOOPFORGE_BROWSER_PYTHON", python);
        let _bridge_guard =
            EnvVarGuard::set("LOOPFORGE_BROWSER_BRIDGE_PATH", bridge_path.as_os_str());

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
    async fn web_fetch_truncation_preserves_head_and_tail() {
        async fn handler() -> String {
            let head = "HEAD_MARKER";
            let tail = "TAIL_MARKER";
            let filler = "A".repeat(5000);
            format!("{head}\n{filler}\n{tail}\n")
        }

        let app = Router::new().route("/", get(handler));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let tmp = tempfile::tempdir().unwrap();
        let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();

        let out = tools
            .call(
                "web_fetch",
                &serde_json::json!({
                    "url": format!("http://{addr}/"),
                    "allow_private": true,
                    "max_bytes": 200,
                })
                .to_string(),
            )
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let body = v.get("body").and_then(|v| v.as_str()).unwrap_or("");

        assert!(body.contains("HEAD_MARKER"), "{body}");
        assert!(body.contains("TAIL_MARKER"), "{body}");
        assert_eq!(
            v.get("truncated").and_then(|v| v.as_bool()),
            Some(true),
            "{v}"
        );

        server.abort();
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
        let _backend_guard = EnvVarGuard::set("LOOPFORGE_BROWSER_BACKEND", "playwright");
        let _python_guard = EnvVarGuard::set("LOOPFORGE_BROWSER_PYTHON", python);
        let _bridge_guard =
            EnvVarGuard::set("LOOPFORGE_BROWSER_BRIDGE_PATH", bridge_path.as_os_str());

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
history = []
scroll_x = 0
scroll_y = 0

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    cmd = json.loads(line)
    action = cmd.get("action", "")
    if action == "Navigate":
        current_url = cmd.get("url", "")
        history.append(current_url)
        resp = {"success": True, "data": {"title": "Stub", "url": current_url, "headless": headless}}
    elif action == "Back":
        if len(history) >= 2:
            history.pop()
            current_url = history[-1]
        resp = {"success": True, "data": {"title": "Stub", "url": current_url}}
    elif action == "Scroll":
        direction = cmd.get("direction", "down")
        amount = int(cmd.get("amount") or 0)
        if direction == "down":
            scroll_y += amount
        elif direction == "up":
            scroll_y -= amount
        elif direction == "right":
            scroll_x += amount
        elif direction == "left":
            scroll_x -= amount
        resp = {"success": True, "data": {"scrollX": scroll_x, "scrollY": scroll_y}}
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
    elif action == "RunJs":
        resp = {"success": True, "data": {"result": 2, "expression": cmd.get("expression", "")}}
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
