use anyhow::Context;

use crate::browser_cdp::session::helpers::KeyEvent;
use crate::browser_cdp::session::CdpBrowserSession;

pub(super) async fn dispatch_known_key(
    session: &CdpBrowserSession,
    event: &KeyEvent,
) -> anyhow::Result<()> {
    dispatch_key_event(session, "keyDown", event)
        .await
        .context("Input.dispatchKeyEvent keyDown")?;
    dispatch_key_event(session, "keyUp", event)
        .await
        .context("Input.dispatchKeyEvent keyUp")?;
    Ok(())
}

async fn dispatch_key_event(
    session: &CdpBrowserSession,
    event_type: &str,
    event: &KeyEvent,
) -> anyhow::Result<()> {
    session
        .cdp
        .send(
            "Input.dispatchKeyEvent",
            super::payload::key_event_payload(event_type, event.key, event.code, event.vkey),
        )
        .await
        .map(|_| ())
}
