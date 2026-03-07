use std::time::Duration;

use crate::defs::resolve_host_ips;
use crate::{extract_between, is_forbidden_ip, Toolset, TOOL_OUTPUT_MIDDLE_OMISSION_MARKER};
use anyhow::{bail, Context};

impl Toolset {
    pub(crate) async fn web_search(
        &self,
        query: &str,
        max_results: Option<u32>,
    ) -> anyhow::Result<String> {
        if query.trim().is_empty() {
            bail!("query is empty");
        }

        let max_results = max_results.unwrap_or(5).clamp(1, 20) as usize;
        let resp = self
            .http
            .get("https://html.duckduckgo.com/html/")
            .query(&[("q", query)])
            .header("User-Agent", "Mozilla/5.0 (compatible; LoopForge/0.1)")
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

    pub(crate) async fn a2a_discover(
        &self,
        url: &str,
        allow_private: bool,
    ) -> anyhow::Result<String> {
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
            .header("User-Agent", "LoopForge/0.1 A2A")
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

    pub(crate) async fn a2a_send(
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
            .header("User-Agent", "LoopForge/0.1 A2A")
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

    pub(crate) async fn web_fetch(
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
        let (body, bytes_returned) = if !truncated {
            (String::from_utf8_lossy(&bytes).to_string(), bytes.len())
        } else {
            let marker = TOOL_OUTPUT_MIDDLE_OMISSION_MARKER.as_bytes();
            if max_bytes <= marker.len() + 2 {
                let slice = &bytes[..max_bytes];
                (String::from_utf8_lossy(slice).to_string(), slice.len())
            } else {
                let budget = max_bytes.saturating_sub(marker.len());
                let tail_budget = (budget / 4).max(1);
                let head_budget = budget.saturating_sub(tail_budget).max(1);

                let head_slice = &bytes[..head_budget.min(bytes.len())];
                let tail_slice = &bytes[bytes.len().saturating_sub(tail_budget)..];

                let mut out = Vec::with_capacity(max_bytes);
                out.extend_from_slice(head_slice);
                out.extend_from_slice(marker);
                out.extend_from_slice(tail_slice);
                (String::from_utf8_lossy(&out).to_string(), out.len())
            }
        };

        Ok(serde_json::json!({
            "status": status,
            "content_type": content_type,
            "body": body,
            "truncated": truncated,
            "bytes": bytes_returned,
            "total_bytes": bytes.len(),
        })
        .to_string())
    }

    pub(crate) async fn pdf_extract(
        &self,
        user_path: &str,
        pages_spec: Option<&str>,
        max_pages: Option<u64>,
        max_chars: Option<u64>,
    ) -> anyhow::Result<String> {
        let path = self.resolve_workspace_path(user_path)?;

        let meta = std::fs::metadata(&path).with_context(|| format!("stat {}", path.display()))?;
        if meta.len() > 20 * 1024 * 1024 {
            bail!("pdf too large: {} bytes", meta.len());
        }

        let ext_ok = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("pdf"))
            .unwrap_or(false);
        if !ext_ok {
            bail!("expected a .pdf file: {user_path}");
        }

        let max_pages = max_pages.unwrap_or(10).clamp(1, 50) as usize;
        let max_chars = max_chars.unwrap_or(12_000).clamp(1, 50_000) as usize;

        let path_for_extract = path.clone();
        let page_texts = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<String>> {
            pdf_extract::extract_text_by_pages(&path_for_extract)
                .map_err(|e| anyhow::anyhow!("pdf extract failed: {e}"))
        })
        .await
        .context("join pdf extract task")??;

        let total_pages = page_texts.len();
        let selected_page_numbers = match pages_spec {
            Some(spec) => Some(Self::parse_pdf_pages_selector(spec)?),
            None => None,
        };

        let selected_pages = if let Some(requested) = selected_page_numbers.as_ref() {
            let mut out = Vec::with_capacity(requested.len());
            for &page_no in requested {
                if page_no == 0 || page_no > total_pages {
                    bail!("page out of range: {page_no} (valid range: 1..={total_pages})");
                }
                out.push(page_texts[page_no - 1].clone());
            }
            out
        } else {
            page_texts
        };

        let pages_extracted = selected_pages.len().min(max_pages);
        let combined = selected_pages
            .into_iter()
            .take(max_pages)
            .collect::<Vec<_>>()
            .join("\n\n");

        let mut iter = combined.chars();
        let text: String = iter.by_ref().take(max_chars).collect();
        let truncated = iter.next().is_some();

        Ok(serde_json::json!({
            "path": user_path,
            "text": text,
            "truncated": truncated,
            "bytes": meta.len(),
            "pages_total": total_pages,
            "pages": pages_spec,
            "pages_extracted": pages_extracted,
        })
        .to_string())
    }

    pub(crate) fn parse_pdf_pages_selector(spec: &str) -> anyhow::Result<Vec<usize>> {
        let spec = spec.trim();
        if spec.is_empty() {
            bail!("pages is empty");
        }

        let mut out = Vec::new();
        for part in spec.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            if let Some((start, end)) = part.split_once('-') {
                let start: usize = start.trim().parse().context("parse pages range start")?;
                let end: usize = end.trim().parse().context("parse pages range end")?;
                if start == 0 || end == 0 {
                    bail!("pages are 1-indexed (got {part})");
                }
                if end < start {
                    bail!("pages range must be ascending (got {part})");
                }
                for n in start..=end {
                    out.push(n);
                }
            } else {
                let n: usize = part.parse().context("parse page number")?;
                if n == 0 {
                    bail!("pages are 1-indexed (got 0)");
                }
                out.push(n);
            }
        }

        if out.is_empty() {
            bail!("pages selection is empty");
        }
        Ok(out)
    }

    pub(crate) fn location_get(&self) -> anyhow::Result<String> {
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
            "note": "Exact geolocation is not available; LoopForge only reports environment metadata.",
        })
        .to_string())
    }
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
