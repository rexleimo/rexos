use anyhow::{bail, Context};
use std::net::IpAddr;

use crate::is_forbidden_ip;

use rexos_llm::openai_compat::{ToolDefinition, ToolFunctionDefinition};
#[derive(Debug, serde::Deserialize)]
pub(crate) struct BrowserNavigateArgs {
    pub(crate) url: String,
    #[serde(default)]
    pub(crate) timeout_ms: Option<u64>,
    #[serde(default)]
    pub(crate) allow_private: bool,
    #[serde(default)]
    pub(crate) headless: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct BrowserRunJsArgs {
    pub(crate) expression: String,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct BrowserClickArgs {
    pub(crate) selector: String,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct BrowserTypeArgs {
    pub(crate) selector: String,
    pub(crate) text: String,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct BrowserPressKeyArgs {
    pub(crate) key: String,
    #[serde(default)]
    pub(crate) selector: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct BrowserScrollArgs {
    #[serde(default)]
    pub(crate) direction: Option<String>,
    #[serde(default)]
    pub(crate) amount: Option<i64>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct BrowserWaitArgs {
    pub(crate) selector: String,
    #[serde(default)]
    pub(crate) timeout_ms: Option<u64>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct BrowserWaitForArgs {
    #[serde(default)]
    pub(crate) selector: Option<String>,
    #[serde(default)]
    pub(crate) text: Option<String>,
    #[serde(default)]
    pub(crate) timeout_ms: Option<u64>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct BrowserScreenshotArgs {
    #[serde(default)]
    pub(crate) path: Option<String>,
}

pub(crate) fn core_tool_defs() -> Vec<ToolDefinition> {
    vec![
        browser_navigate_def(),
        browser_back_def(),
        browser_scroll_def(),
        browser_click_def(),
        browser_type_def(),
        browser_press_key_def(),
        browser_wait_def(),
        browser_wait_for_def(),
        browser_read_page_def(),
        browser_run_js_def(),
        browser_screenshot_def(),
        browser_close_def(),
    ]
}

pub(crate) fn compat_tool_defs() -> Vec<ToolDefinition> {
    Vec::new()
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

fn browser_back_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "browser_back".to_string(),
            description: "Go back in browser history.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": [],
                "additionalProperties": false
            }),
        },
    }
}

fn browser_scroll_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "browser_scroll".to_string(),
            description: "Scroll the current page.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "direction": { "type": "string", "description": "Scroll direction: down/up/left/right (default down).", "enum": ["down", "up", "left", "right"] },
                    "amount": { "type": "integer", "description": "Scroll amount in pixels (default 600).", "minimum": 0 }
                },
                "required": [],
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

fn browser_wait_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "browser_wait".to_string(),
            description: "Wait for a CSS selector to appear on the page.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "selector": { "type": "string", "description": "CSS selector to wait for." },
                    "timeout_ms": { "type": "integer", "description": "Optional timeout in milliseconds.", "minimum": 1 }
                },
                "required": ["selector"],
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

fn browser_run_js_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "browser_run_js".to_string(),
            description: "Run a JavaScript expression on the current page and return the result."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "expression": { "type": "string", "description": "JavaScript expression to evaluate." }
                },
                "required": ["expression"],
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
                    "path": { "type": "string", "description": "Relative output path (default .loopforge/browser/screenshot.png)." }
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

pub(crate) async fn resolve_host_ips(host: &str, port: u16) -> anyhow::Result<Vec<IpAddr>> {
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

pub(crate) async fn ensure_browser_url_allowed(
    url: &str,
    allow_private: bool,
) -> anyhow::Result<()> {
    let url = reqwest::Url::parse(url).context("parse url")?;

    match url.scheme() {
        "http" | "https" => {}
        // Safe, non-network internal pages we still want to allow for screenshots/debugging.
        "about" if url.as_str() == "about:blank" => return Ok(()),
        "chrome-error" if matches!(url.host_str(), Some("chromewebdata")) => return Ok(()),
        _ => bail!("only http/https urls are allowed"),
    }

    if allow_private {
        return Ok(());
    }

    let host = url.host_str().context("url missing host")?;
    let port = url.port_or_known_default().context("url missing port")?;

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
