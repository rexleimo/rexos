use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context};
use dashmap::DashMap;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::io::AsyncBufReadExt;
use tokio::sync::{oneshot, Mutex};
use tokio_tungstenite::tungstenite::Message as WsMessage;

type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

const CDP_CONNECT_TIMEOUT_SECS: u64 = 15;
const CDP_COMMAND_TIMEOUT_SECS: u64 = 30;
const PAGE_LOAD_POLL_INTERVAL_MS: u64 = 200;
const PAGE_LOAD_MAX_POLLS: u32 = 150; // 30 seconds

pub struct CdpBrowserSession {
    pub headless: bool,
    pub allow_private: bool,
    process: Option<tokio::process::Child>,
    user_data_dir: Option<PathBuf>,
    cdp: CdpConnection,
}

impl CdpBrowserSession {
    pub async fn connect_or_launch(
        http: reqwest::Client,
        headless: bool,
        allow_private: bool,
    ) -> anyhow::Result<Self> {
        if let Ok(v) = std::env::var("REXOS_BROWSER_CDP_HTTP") {
            let v = v.trim().to_string();
            if !v.is_empty() {
                return Self::connect_remote(http, &v, headless, allow_private).await;
            }
        }

        Self::launch_local(http, headless, allow_private).await
    }

    async fn connect_remote(
        http: reqwest::Client,
        base_http: &str,
        headless: bool,
        allow_private: bool,
    ) -> anyhow::Result<Self> {
        let base = reqwest::Url::parse(base_http).context("parse REXOS_BROWSER_CDP_HTTP")?;
        let page_ws = find_or_create_page_ws(&http, &base).await?;
        let cdp = CdpConnection::connect(&page_ws).await?;

        let _ = cdp.send("Page.enable", serde_json::json!({})).await;
        let _ = cdp.send("Runtime.enable", serde_json::json!({})).await;

        Ok(Self {
            headless,
            allow_private,
            process: None,
            user_data_dir: None,
            cdp,
        })
    }

    async fn launch_local(
        http: reqwest::Client,
        headless: bool,
        allow_private: bool,
    ) -> anyhow::Result<Self> {
        let chrome_path = find_chromium()?;

        let port = pick_unused_port().context("pick unused port")?;

        let user_data_dir = std::env::temp_dir().join(format!("rexos-chrome-{}", uuid::Uuid::new_v4()));
        let user_data_dir_arg = user_data_dir.to_string_lossy().to_string();

        let mut args = vec![
            format!("--remote-debugging-port={port}"),
            "--remote-debugging-address=127.0.0.1".to_string(),
            "--no-first-run".to_string(),
            "--no-default-browser-check".to_string(),
            "--disable-extensions".to_string(),
            "--disable-background-networking".to_string(),
            "--disable-sync".to_string(),
            "--disable-translate".to_string(),
            "--disable-features=TranslateUI".to_string(),
            "--metrics-recording-only".to_string(),
            "--disable-popup-blocking".to_string(),
            "--window-size=1280,720".to_string(),
            format!("--user-data-dir={user_data_dir_arg}"),
            "about:blank".to_string(),
        ];

        if headless {
            args.insert(0, "--headless=new".to_string());
            args.push("--disable-gpu".to_string());
        }

        if std::env::var("REXOS_BROWSER_NO_SANDBOX")
            .ok()
            .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(false)
        {
            args.push("--no-sandbox".to_string());
            args.push("--disable-setuid-sandbox".to_string());
        }

        let mut cmd = tokio::process::Command::new(&chrome_path);
        cmd.args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .env_clear();

        // Pass through only minimal env
        for key in [
            "PATH",
            "HOME",
            "USER",
            "USERPROFILE",
            "SYSTEMROOT",
            "TEMP",
            "TMP",
            "TMPDIR",
            "APPDATA",
            "LOCALAPPDATA",
            "XDG_CONFIG_HOME",
            "XDG_CACHE_HOME",
            "DISPLAY",
            "WAYLAND_DISPLAY",
        ] {
            if let Ok(v) = std::env::var(key) {
                cmd.env(key, v);
            }
        }

        let mut child = cmd
            .spawn()
            .with_context(|| format!("launch Chromium at {}", chrome_path.display()))?;

        // Ensure we can see the DevTools endpoint
        let stderr = child.stderr.take().context("capture Chromium stderr")?;
        let _ws_url = read_devtools_url(stderr).await?;

        let base = reqwest::Url::parse(&format!("http://127.0.0.1:{port}"))
            .context("parse local CDP base url")?;
        let page_ws = find_or_create_page_ws(&http, &base).await?;
        let cdp = CdpConnection::connect(&page_ws).await?;

        let _ = cdp.send("Page.enable", serde_json::json!({})).await;
        let _ = cdp.send("Runtime.enable", serde_json::json!({})).await;

        Ok(Self {
            headless,
            allow_private,
            process: Some(child),
            user_data_dir: Some(user_data_dir),
            cdp,
        })
    }

    pub async fn navigate(&self, url: &str) -> anyhow::Result<Value> {
        self.cdp
            .send("Page.navigate", serde_json::json!({ "url": url }))
            .await
            .context("Page.navigate")?;

        wait_for_load(&self.cdp).await;
        page_info(&self.cdp).await
    }

    pub async fn click(&self, selector: &str) -> anyhow::Result<Value> {
        let sel_json = serde_json::to_string(selector).unwrap_or_default();
        let js = format!(
            r#"(() => {{
    let sel = {sel_json};
    let el = document.querySelector(sel);
    if (!el) {{
        const all = document.querySelectorAll('a, button, [role="button"], input[type="submit"], [onclick]');
        const lower = sel.toLowerCase();
        for (const e of all) {{
            if (e.textContent.trim().toLowerCase().includes(lower)) {{ el = e; break; }}
        }}
    }}
    if (!el) return JSON.stringify({{success: false, error: 'Element not found: ' + sel}});
    el.scrollIntoView({{block: 'center'}});
    el.click();
    return JSON.stringify({{success: true, tag: el.tagName, text: el.textContent.substring(0, 100).trim()}});
}})()"#
        );

        let val = self.cdp.run_js(&js).await.context("click js")?;
        let parsed: Value = val
            .as_str()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(val);
        if parsed["success"].as_bool() == Some(false) {
            let msg = parsed["error"]
                .as_str()
                .unwrap_or("click failed")
                .to_string();
            bail!(msg);
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
        wait_for_load(&self.cdp).await;
        page_info(&self.cdp).await.or(Ok(parsed))
    }

    pub async fn type_text(&self, selector: &str, text: &str) -> anyhow::Result<Value> {
        let sel_json = serde_json::to_string(selector).unwrap_or_default();
        let text_json = serde_json::to_string(text).unwrap_or_default();
        let js = format!(
            r#"(() => {{
    let sel = {sel_json};
    let txt = {text_json};
    let el = document.querySelector(sel);
    if (!el) return JSON.stringify({{success: false, error: 'Input not found: ' + sel}});
    el.focus();
    el.value = txt;
    el.dispatchEvent(new Event('input', {{bubbles: true}}));
    el.dispatchEvent(new Event('change', {{bubbles: true}}));
    return JSON.stringify({{success: true, selector: sel, typed: txt.length + ' chars'}});
}})()"#
        );

        let val = self.cdp.run_js(&js).await.context("type js")?;
        let parsed: Value = val
            .as_str()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(val);
        if parsed["success"].as_bool() == Some(false) {
            let msg = parsed["error"]
                .as_str()
                .unwrap_or("type failed")
                .to_string();
            bail!(msg);
        }
        Ok(parsed)
    }

    pub async fn press_key(&self, selector: Option<&str>, key: &str) -> anyhow::Result<Value> {
        if let Some(selector) = selector {
            let sel_json = serde_json::to_string(selector).unwrap_or_default();
            let js = format!(
                r#"(() => {{
    let sel = {sel_json};
    let el = document.querySelector(sel);
    if (el) el.focus();
    return JSON.stringify({{focused: !!el, selector: sel}});
}})()"#
            );
            let _ = self.cdp.run_js(&js).await;
        }

        if let Some(event) = key_event_fields(key) {
            self.cdp
                .send(
                    "Input.dispatchKeyEvent",
                    serde_json::json!({
                        "type": "keyDown",
                        "key": event.key,
                        "code": event.code,
                        "windowsVirtualKeyCode": event.vkey,
                        "nativeVirtualKeyCode": event.vkey,
                    }),
                )
                .await
                .context("Input.dispatchKeyEvent keyDown")?;
            self.cdp
                .send(
                    "Input.dispatchKeyEvent",
                    serde_json::json!({
                        "type": "keyUp",
                        "key": event.key,
                        "code": event.code,
                        "windowsVirtualKeyCode": event.vkey,
                        "nativeVirtualKeyCode": event.vkey,
                    }),
                )
                .await
                .context("Input.dispatchKeyEvent keyUp")?;
        } else {
            // Best-effort fallback (not all sites will respond to synthetic events).
            let key_json = serde_json::to_string(key).unwrap_or_default();
            let js = format!(
                r#"(() => {{
    let k = {key_json};
    let el = document.activeElement || document.body;
    try {{
      el.dispatchEvent(new KeyboardEvent('keydown', {{key: k, bubbles: true}}));
      el.dispatchEvent(new KeyboardEvent('keyup', {{key: k, bubbles: true}}));
    }} catch (e) {{}}
    if (k === 'Enter' && el) {{
      try {{
        const form = el.form || el.closest?.('form');
        if (form?.requestSubmit) form.requestSubmit();
      }} catch (e) {{}}
    }}
    return JSON.stringify({{ok: true, key: k}});
}})()"#
            );
            let _ = self.cdp.run_js(&js).await;
        }

        tokio::time::sleep(Duration::from_millis(250)).await;
        wait_for_load(&self.cdp).await;
        let info = page_info(&self.cdp).await.unwrap_or_else(|_| serde_json::json!({}));
        let mut obj = info.as_object().cloned().unwrap_or_default();
        obj.insert("key".to_string(), Value::String(key.to_string()));
        obj.insert(
            "selector".to_string(),
            selector
                .map(|s| Value::String(s.to_string()))
                .unwrap_or(Value::Null),
        );
        Ok(Value::Object(obj))
    }

    pub async fn wait_for(
        &self,
        selector: Option<&str>,
        text: Option<&str>,
        timeout_ms: Option<u64>,
    ) -> anyhow::Result<Value> {
        let selector = selector.map(|s| s.trim()).filter(|s| !s.is_empty());
        let text = text.map(|s| s.trim()).filter(|s| !s.is_empty());
        if selector.is_none() && text.is_none() {
            bail!("wait_for requires selector or text");
        }

        let max_ms = timeout_ms.unwrap_or(30_000).min(30_000);
        let polls = (max_ms / PAGE_LOAD_POLL_INTERVAL_MS).max(1);

        for _ in 0..polls {
            let mut waited_for = serde_json::Map::new();

            if let Some(sel) = selector {
                let sel_json = serde_json::to_string(sel).unwrap_or_default();
                let js = format!("document.querySelector({sel_json}) ? 'found' : null");
                let found = self
                    .cdp
                    .run_js(&js)
                    .await
                    .ok()
                    .map(|v| v.as_str() == Some("found"))
                    .unwrap_or(false);
                if found {
                    waited_for.insert("selector".to_string(), Value::String(sel.to_string()));
                }
            }

            if let Some(t) = text {
                let text_json = serde_json::to_string(t).unwrap_or_default();
                let js = format!(
                    "document.body && document.body.innerText && document.body.innerText.includes({text_json}) ? 'found' : null"
                );
                let found = self
                    .cdp
                    .run_js(&js)
                    .await
                    .ok()
                    .map(|v| v.as_str() == Some("found"))
                    .unwrap_or(false);
                if found {
                    waited_for.insert("text".to_string(), Value::String(t.to_string()));
                }
            }

            let selector_ok = selector.is_none() || waited_for.contains_key("selector");
            let text_ok = text.is_none() || waited_for.contains_key("text");
            if selector_ok && text_ok && !waited_for.is_empty() {
                let info = page_info(&self.cdp).await.unwrap_or(Value::Null);
                return Ok(serde_json::json!({
                    "waited_for": waited_for,
                    "timeout_ms": max_ms,
                    "title": info.get("title").cloned().unwrap_or(Value::Null),
                    "url": info.get("url").cloned().unwrap_or(Value::Null),
                }));
            }

            tokio::time::sleep(Duration::from_millis(PAGE_LOAD_POLL_INTERVAL_MS)).await;
        }

        bail!("timed out waiting ({}ms)", max_ms);
    }

    pub async fn read_page(&self) -> anyhow::Result<Value> {
        let val = self.cdp.run_js(EXTRACT_CONTENT_JS).await.context("read page js")?;
        let parsed: Value = val
            .as_str()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(val);
        Ok(parsed)
    }

    pub async fn screenshot(&self) -> anyhow::Result<Value> {
        let result = self
            .cdp
            .send("Page.captureScreenshot", serde_json::json!({ "format": "png" }))
            .await
            .context("Page.captureScreenshot")?;
        let b64 = result.get("data").and_then(|v| v.as_str()).unwrap_or("");
        let url = self
            .cdp
            .run_js("location.href")
            .await
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default();
        Ok(serde_json::json!({
            "format": "png",
            "url": url,
            "image_base64": b64,
        }))
    }

    pub async fn current_url(&self) -> anyhow::Result<String> {
        let v = self.cdp.run_js("location.href").await.context("location.href")?;
        Ok(v.as_str().unwrap_or("").to_string())
    }

    pub async fn close(&mut self) {
        if let Some(mut child) = self.process.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        if let Some(dir) = self.user_data_dir.take() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

impl Drop for CdpBrowserSession {
    fn drop(&mut self) {
        if let Some(child) = &mut self.process {
            let _ = child.start_kill();
        }
        if let Some(dir) = self.user_data_dir.take() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

struct KeyEvent {
    key: &'static str,
    code: &'static str,
    vkey: i32,
}

fn key_event_fields(key: &str) -> Option<KeyEvent> {
    match key {
        "Enter" => Some(KeyEvent {
            key: "Enter",
            code: "Enter",
            vkey: 13,
        }),
        "Tab" => Some(KeyEvent {
            key: "Tab",
            code: "Tab",
            vkey: 9,
        }),
        "Escape" | "Esc" => Some(KeyEvent {
            key: "Escape",
            code: "Escape",
            vkey: 27,
        }),
        "ArrowUp" => Some(KeyEvent {
            key: "ArrowUp",
            code: "ArrowUp",
            vkey: 38,
        }),
        "ArrowDown" => Some(KeyEvent {
            key: "ArrowDown",
            code: "ArrowDown",
            vkey: 40,
        }),
        "ArrowLeft" => Some(KeyEvent {
            key: "ArrowLeft",
            code: "ArrowLeft",
            vkey: 37,
        }),
        "ArrowRight" => Some(KeyEvent {
            key: "ArrowRight",
            code: "ArrowRight",
            vkey: 39,
        }),
        _ => None,
    }
}

fn pick_unused_port() -> anyhow::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").context("bind 127.0.0.1:0")?;
    let port = listener.local_addr().context("local_addr")?.port();
    Ok(port)
}

async fn read_devtools_url(stderr: tokio::process::ChildStderr) -> anyhow::Result<String> {
    let reader = tokio::io::BufReader::new(stderr);
    let mut lines = reader.lines();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(CDP_CONNECT_TIMEOUT_SECS);

    loop {
        let line = tokio::time::timeout_at(deadline, lines.next_line())
            .await
            .map_err(|_| anyhow::anyhow!("timed out waiting for Chromium to start"))?
            .context("read Chromium stderr")?;

        match line {
            Some(l) if l.contains("DevTools listening on") => {
                let url = l
                    .split("DevTools listening on ")
                    .nth(1)
                    .context("malformed DevTools URL line")?
                    .trim()
                    .to_string();
                return Ok(url);
            }
            Some(_) => continue,
            None => bail!("Chromium exited before printing DevTools URL"),
        }
    }
}

async fn find_or_create_page_ws(http: &reqwest::Client, base: &reqwest::Url) -> anyhow::Result<String> {
    // Prefer /json/new (creates a fresh tab).
    let new_url = base.join("/json/new").context("join /json/new")?;
    if let Ok(resp) = http.get(new_url).send().await {
        if resp.status().is_success() {
            let v: Value = resp.json().await.unwrap_or(Value::Null);
            if let Some(ws) = v.get("webSocketDebuggerUrl").and_then(|v| v.as_str()) {
                return Ok(ws.to_string());
            }
        }
    }

    let list_url = base.join("/json/list").context("join /json/list")?;
    for attempt in 0..10 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(300)).await;
        }

        let resp = http.get(list_url.clone()).send().await;
        let Ok(resp) = resp else { continue };
        if !resp.status().is_success() {
            continue;
        }

        let targets: Vec<Value> = resp.json().await.unwrap_or_default();
        for target in &targets {
            if target.get("type").and_then(|v| v.as_str()) == Some("page") {
                if let Some(ws) = target.get("webSocketDebuggerUrl").and_then(|v| v.as_str()) {
                    return Ok(ws.to_string());
                }
            }
        }
    }

    bail!("no page target found at {}", base);
}

async fn wait_for_load(cdp: &CdpConnection) {
    for _ in 0..PAGE_LOAD_MAX_POLLS {
        if let Ok(val) = cdp.run_js("document.readyState").await {
            let state = val.as_str().unwrap_or("");
            if state == "complete" || state == "interactive" {
                return;
            }
        }
        tokio::time::sleep(Duration::from_millis(PAGE_LOAD_POLL_INTERVAL_MS)).await;
    }
}

async fn page_info(cdp: &CdpConnection) -> anyhow::Result<Value> {
    let info = cdp
        .run_js("JSON.stringify({title: document.title, url: location.href})")
        .await
        .context("page info js")?;
    let parsed: Value = info
        .as_str()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(info);

    Ok(serde_json::json!({
        "title": parsed.get("title").cloned().unwrap_or(Value::Null),
        "url": parsed.get("url").cloned().unwrap_or(Value::Null),
    }))
}

// ── CDP connection ─────────────────────────────────────────────────────────

struct CdpConnection {
    write: Arc<Mutex<SplitSink<WsStream, WsMessage>>>,
    pending: Arc<DashMap<u64, oneshot::Sender<anyhow::Result<Value>>>>,
    next_id: AtomicU64,
    _reader_handle: tokio::task::JoinHandle<()>,
}

impl CdpConnection {
    async fn connect(ws_url: &str) -> anyhow::Result<Self> {
        let (stream, _) = tokio::time::timeout(
            Duration::from_secs(CDP_CONNECT_TIMEOUT_SECS),
            tokio_tungstenite::connect_async(ws_url),
        )
        .await
        .map_err(|_| anyhow::anyhow!("CDP WebSocket connect timed out: {ws_url}"))?
        .map_err(|e| anyhow::anyhow!("CDP WebSocket connect failed: {e}"))?;

        let (write, read) = stream.split();
        let write = Arc::new(Mutex::new(write));
        let pending: Arc<DashMap<u64, oneshot::Sender<anyhow::Result<Value>>>> =
            Arc::new(DashMap::new());

        let reader_pending = Arc::clone(&pending);
        let reader_handle = tokio::spawn(Self::reader_loop(read, reader_pending));

        Ok(Self {
            write,
            pending,
            next_id: AtomicU64::new(1),
            _reader_handle: reader_handle,
        })
    }

    async fn reader_loop(
        mut read: SplitStream<WsStream>,
        pending: Arc<DashMap<u64, oneshot::Sender<anyhow::Result<Value>>>>,
    ) {
        while let Some(msg) = read.next().await {
            let text = match msg {
                Ok(WsMessage::Text(t)) => t.to_string(),
                Ok(WsMessage::Close(_)) => break,
                Err(_) => break,
                _ => continue,
            };

            let json: Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(_) => continue,
            };

            if let Some(id) = json.get("id").and_then(|v| v.as_u64()) {
                if let Some((_, sender)) = pending.remove(&id) {
                    if let Some(error) = json.get("error") {
                        let msg = error
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("CDP error")
                            .to_string();
                        let _ = sender.send(Err(anyhow::anyhow!(msg)));
                    } else {
                        let result = json.get("result").cloned().unwrap_or(Value::Null);
                        let _ = sender.send(Ok(result));
                    }
                }
            }
        }
    }

    async fn send(&self, method: &str, params: Value) -> anyhow::Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.pending.insert(id, tx);

        let msg = serde_json::json!({ "id": id, "method": method, "params": params });
        self.write
            .lock()
            .await
            .send(WsMessage::Text(msg.to_string().into()))
            .await
            .context("CDP send")?;

        match tokio::time::timeout(Duration::from_secs(CDP_COMMAND_TIMEOUT_SECS), rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => bail!("CDP response channel closed"),
            Err(_) => {
                self.pending.remove(&id);
                bail!("CDP command timed out");
            }
        }
    }

    async fn run_js(&self, expression: &str) -> anyhow::Result<Value> {
        let result = self
            .send(
                "Runtime.evaluate",
                serde_json::json!({
                    "expression": expression,
                    "returnByValue": true,
                    "awaitPromise": true,
                }),
            )
            .await?;

        if let Some(desc) = result
            .get("exceptionDetails")
            .and_then(|e| e.get("text"))
            .and_then(|t| t.as_str())
        {
            bail!("JS error: {desc}");
        }

        Ok(result
            .get("result")
            .and_then(|r| r.get("value"))
            .cloned()
            .unwrap_or(Value::Null))
    }
}

impl Drop for CdpConnection {
    fn drop(&mut self) {
        self._reader_handle.abort();
    }
}

// ── Chromium discovery ─────────────────────────────────────────────────────

fn find_chromium() -> anyhow::Result<PathBuf> {
    if let Ok(path) = std::env::var("REXOS_BROWSER_CHROME_PATH") {
        let p = PathBuf::from(path.trim());
        if p.exists() {
            return Ok(p);
        }
        bail!(
            "REXOS_BROWSER_CHROME_PATH does not exist: {}",
            p.display()
        );
    }

    if let Ok(path) = std::env::var("CHROME_PATH") {
        let p = PathBuf::from(path.trim());
        if p.exists() {
            return Ok(p);
        }
    }

    for candidate in chromium_candidates() {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    for name in [
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
        "chrome",
        "msedge",
    ] {
        if let Some(p) = find_in_path(name) {
            return Ok(p);
        }
    }

    bail!("could not find Chrome/Chromium. Install Chrome/Chromium or set REXOS_BROWSER_CHROME_PATH.")
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let exts: Vec<&str> = if cfg!(windows) {
        vec![".exe", ""]
    } else {
        vec![""]
    };

    for dir in std::env::split_paths(&path) {
        for ext in &exts {
            let cand = dir.join(format!("{name}{ext}"));
            if cand.exists() {
                return Some(cand);
            }
        }
    }
    None
}

fn chromium_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();

    #[cfg(target_os = "macos")]
    {
        out.push("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome".into());
        out.push("/Applications/Chromium.app/Contents/MacOS/Chromium".into());
        out.push("/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge".into());
    }

    #[cfg(target_os = "linux")]
    {
        out.push("/usr/bin/google-chrome".into());
        out.push("/usr/bin/google-chrome-stable".into());
        out.push("/usr/bin/chromium".into());
        out.push("/usr/bin/chromium-browser".into());
    }

    #[cfg(windows)]
    {
        let program_files = std::env::var_os("ProgramFiles").map(PathBuf::from);
        let program_files_x86 = std::env::var_os("ProgramFiles(x86)").map(PathBuf::from);
        let local_app_data = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);

        for base in [program_files, program_files_x86, local_app_data] {
            let Some(base) = base else { continue };
            out.push(base.join("Google/Chrome/Application/chrome.exe"));
            out.push(base.join("Chromium/Application/chrome.exe"));
            out.push(base.join("Microsoft/Edge/Application/msedge.exe"));
        }
    }

    out
}

const EXTRACT_CONTENT_JS: &str = r#"(() => {
  function clean(s) {
    return (s || '').replace(/\s+/g, ' ').trim();
  }
  let title = document.title || '';
  let url = location.href || '';
  let body = '';
  try {
    body = document.body ? document.body.innerText : '';
  } catch (e) {
    body = '';
  }
  body = body || '';
  const max = 50000;
  let truncated = false;
  if (body.length > max) {
    body = body.substring(0, max);
    truncated = true;
  }
  return JSON.stringify({
    title: clean(title),
    url: url,
    content: body + (truncated ? `\n\n[Truncated — ${max} chars]` : ''),
  });
})()"#;
