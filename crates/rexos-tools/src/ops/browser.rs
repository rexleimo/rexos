use anyhow::{bail, Context};
use base64::Engine as _;

use crate::browser_cdp::CdpBrowserSession;
use crate::defs::{ensure_browser_url_allowed, resolve_host_ips};
use crate::{
    browser_backend_default, browser_headless_default, is_forbidden_ip, BrowserBackend,
    BrowserSession, PlaywrightBrowserSession, Toolset,
};

impl Toolset {
    pub(crate) async fn browser_navigate(
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
                    let s = CdpBrowserSession::connect_or_launch(
                        self.http.clone(),
                        headless,
                        allow_private,
                    )
                    .await?;
                    BrowserSession::Cdp(s)
                }
                BrowserBackend::Playwright => BrowserSession::Playwright(
                    PlaywrightBrowserSession::spawn(headless, allow_private).await?,
                ),
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
        if let Some(url) = out.get("url").and_then(|v| v.as_str()) {
            ensure_browser_url_allowed(url, session.allow_private()).await?;
        }
        Ok(out.to_string())
    }

    pub(crate) async fn browser_back(&self) -> anyhow::Result<String> {
        let mut guard = self.browser.lock().await;
        let session = guard
            .as_mut()
            .context("browser session not started; call browser_navigate first")?;
        let out = session.back().await?;
        if let Some(url) = out.get("url").and_then(|v| v.as_str()) {
            ensure_browser_url_allowed(url, session.allow_private()).await?;
        }
        Ok(out.to_string())
    }

    pub(crate) async fn browser_scroll(
        &self,
        direction: Option<&str>,
        amount: Option<i64>,
    ) -> anyhow::Result<String> {
        let direction = direction.unwrap_or("down").trim().to_ascii_lowercase();
        let amount = amount.unwrap_or(600).clamp(0, 50_000);
        if !matches!(direction.as_str(), "down" | "up" | "left" | "right") {
            bail!("invalid direction: {direction} (expected down/up/left/right)");
        }

        let mut guard = self.browser.lock().await;
        let session = guard
            .as_mut()
            .context("browser session not started; call browser_navigate first")?;
        let out = session.scroll(&direction, amount).await?;
        Ok(out.to_string())
    }

    pub(crate) async fn browser_close(&self) -> anyhow::Result<String> {
        let mut guard = self.browser.lock().await;
        if let Some(mut session) = guard.take() {
            session.close().await;
        }
        Ok("ok".to_string())
    }

    pub(crate) async fn browser_click(&self, selector: &str) -> anyhow::Result<String> {
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

    pub(crate) async fn browser_type(&self, selector: &str, text: &str) -> anyhow::Result<String> {
        let mut guard = self.browser.lock().await;
        let session = guard
            .as_mut()
            .context("browser session not started; call browser_navigate first")?;
        let out = session.type_text(selector, text).await?;
        Ok(out.to_string())
    }

    pub(crate) async fn browser_press_key(
        &self,
        selector: Option<&str>,
        key: &str,
    ) -> anyhow::Result<String> {
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

    pub(crate) async fn browser_wait(
        &self,
        selector: &str,
        timeout_ms: Option<u64>,
    ) -> anyhow::Result<String> {
        if selector.trim().is_empty() {
            bail!("selector is empty");
        }

        let mut guard = self.browser.lock().await;
        let session = guard
            .as_mut()
            .context("browser session not started; call browser_navigate first")?;
        let out = session.wait_for(Some(selector), None, timeout_ms).await?;
        if let Some(url) = out.get("url").and_then(|v| v.as_str()) {
            ensure_browser_url_allowed(url, session.allow_private()).await?;
        }
        Ok(out.to_string())
    }

    pub(crate) async fn browser_wait_for(
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

    pub(crate) async fn browser_run_js(&self, expression: &str) -> anyhow::Result<String> {
        if expression.trim().is_empty() {
            bail!("expression is empty");
        }

        if expression.len() > 100_000 {
            bail!("expression too large");
        }

        let mut guard = self.browser.lock().await;
        let session = guard
            .as_mut()
            .context("browser session not started; call browser_navigate first")?;
        let out = session.run_js(expression).await?;
        if let Some(url) = out.get("url").and_then(|v| v.as_str()) {
            ensure_browser_url_allowed(url, session.allow_private()).await?;
        }
        Ok(out.to_string())
    }

    pub(crate) async fn browser_read_page(&self) -> anyhow::Result<String> {
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

    pub(crate) async fn browser_screenshot(&self, path: Option<&str>) -> anyhow::Result<String> {
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

        let rel = path.unwrap_or(".loopforge/browser/screenshot.png");
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
}
