use serde_json::{Map, Value};

use crate::browser_cdp::session::helpers::page_info;
use crate::browser_cdp::session::CdpBrowserSession;

pub(super) fn normalize_wait_arg(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

pub(super) fn poll_count(max_ms: u64, poll_interval_ms: u64) -> u64 {
    (max_ms / poll_interval_ms).max(1)
}

pub(super) fn wait_satisfied(
    waited_for: &Map<String, Value>,
    selector: Option<&str>,
    text: Option<&str>,
) -> bool {
    let selector_ok = selector.is_none() || waited_for.contains_key("selector");
    let text_ok = text.is_none() || waited_for.contains_key("text");
    selector_ok && text_ok && !waited_for.is_empty()
}

pub(super) async fn wait_response(
    session: &CdpBrowserSession,
    waited_for: Map<String, Value>,
    max_ms: u64,
) -> Value {
    let info = page_info(&session.cdp).await.unwrap_or(Value::Null);
    serde_json::json!({
        "waited_for": waited_for,
        "timeout_ms": max_ms,
        "title": info.get("title").cloned().unwrap_or(Value::Null),
        "url": info.get("url").cloned().unwrap_or(Value::Null),
    })
}
