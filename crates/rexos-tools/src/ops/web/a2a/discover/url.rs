use anyhow::Context;
use rexos_kernel::security::SecurityConfig;

pub(super) async fn agent_card_url(
    url: &str,
    allow_private: bool,
    security: &SecurityConfig,
) -> anyhow::Result<reqwest::Url> {
    let mut url = reqwest::Url::parse(url).context("parse url")?;
    url.set_path("/.well-known/agent.json");
    url.set_query(None);
    url.set_fragment(None);
    super::super::super::ensure_remote_url_allowed(
        &url,
        allow_private,
        "a2a_discover",
        "GET",
        security,
    )
    .await?;
    Ok(url)
}
