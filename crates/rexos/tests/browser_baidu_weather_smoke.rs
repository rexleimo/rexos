use rexos::llm::openai_compat::{ChatCompletionRequest, ChatMessage, OpenAiCompatibleClient, Role};
use rexos::tools::Toolset;

fn percent_encode_query(query: &str) -> String {
    let mut out = String::new();
    for b in query.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

#[tokio::test]
#[ignore]
async fn browser_baidu_search_weather_and_summarize_with_ollama_smoke() {
    let model = std::env::var("REXOS_OLLAMA_MODEL").unwrap_or_else(|_| "qwen3:4b".to_string());
    let query =
        std::env::var("REXOS_BAIDU_WEATHER_QUERY").unwrap_or_else(|_| "北京 今天天气".to_string());
    let headless = match std::env::var("REXOS_BROWSER_HEADLESS") {
        Ok(v) => match v.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => false,
        },
        Err(_) => false, // default headed for this smoke test (so you can see the browser window)
    };
    let demo_pause_ms: u64 = std::env::var("REXOS_BROWSER_DEMO_PAUSE_MS")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(if headless { 0 } else { 1500 });

    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().to_path_buf();
    let tools = Toolset::new(workspace.clone()).unwrap();

    // 1) Open Baidu homepage.
    let nav = tools
        .call(
            "browser_navigate",
            &serde_json::json!({
                "url": "https://www.baidu.com",
                "timeout_ms": 30000,
                "headless": headless,
            })
            .to_string(),
        )
        .await
        .expect("browser_navigate should succeed (requires a Chromium-based browser; default backend is CDP)");
    let nav_v: serde_json::Value = serde_json::from_str(&nav).unwrap();
    assert!(
        nav_v
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .contains("百度"),
        "unexpected title: {nav_v}"
    );

    // Save an early screenshot so we have evidence even if the flow changes.
    let _ = tools
        .call(
            "browser_screenshot",
            r#"{ "path": ".rexos/browser/baidu_home.png" }"#,
        )
        .await
        .expect("browser_screenshot (home)");

    // 2) Type query and submit (best-effort). If Baidu hides the search box for automation, fall
    // back to opening the results URL directly.
    let search_box_ready = tools
        .call(
            "browser_wait_for",
            r#"{ "selector": "input[name=\"wd\"]", "timeout_ms": 3000 }"#,
        )
        .await
        .is_ok();

    if search_box_ready {
        let _ = tools
            .call(
                "browser_type",
                &serde_json::json!({
                    "selector": "input[name=\"wd\"]",
                    "text": query,
                })
                .to_string(),
            )
            .await
            .expect("browser_type");
        let _ = tools
            .call(
                "browser_press_key",
                r#"{ "selector": "input[name=\"wd\"]", "key": "Enter" }"#,
            )
            .await
            .expect("browser_press_key");
    } else {
        let results_url = format!(
            "https://www.baidu.com/s?wd={}",
            percent_encode_query(&query)
        );
        let _ = tools
            .call(
                "browser_navigate",
                &serde_json::json!({
                    "url": results_url,
                    "timeout_ms": 30000,
                })
                .to_string(),
            )
            .await
            .expect("browser_navigate (direct results url)");
    }

    // 3) Wait for results container and read page.
    let _ = tools
        .call(
            "browser_wait_for",
            &serde_json::json!({
                "selector": "#content_left",
                "timeout_ms": 30000,
            })
            .to_string(),
        )
        .await
        .expect("browser_wait_for results");
    let page = tools
        .call("browser_read_page", r#"{}"#)
        .await
        .expect("browser_read_page");
    let page_v: serde_json::Value = serde_json::from_str(&page).unwrap();
    let page_url = page_v
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let page_text = page_v
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    assert!(
        page_url.contains("baidu.com") && page_url.contains("wd="),
        "unexpected results url: {page_url:?}"
    );
    assert!(
        page_text.contains("天气"),
        "expected page text to contain '天气' (got len={})",
        page_text.len()
    );

    // 4) Save a screenshot as evidence the browser ran.
    let _ = tools
        .call(
            "browser_screenshot",
            r#"{ "path": ".rexos/browser/baidu_weather.png" }"#,
        )
        .await
        .expect("browser_screenshot");
    let screenshot_path = workspace.join(".rexos/browser/baidu_weather.png");
    let screenshot_bytes = std::fs::read(&screenshot_path).expect("read screenshot");
    assert!(
        screenshot_bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]),
        "screenshot is not a PNG: {}",
        screenshot_path.display()
    );

    if demo_pause_ms > 0 {
        tokio::time::sleep(std::time::Duration::from_millis(demo_pause_ms)).await;
    }
    let _ = tools.call("browser_close", r#"{}"#).await;

    // 5) Ask Ollama (OpenAI-compatible) to extract a concise weather summary from the page text.
    let client = OpenAiCompatibleClient::new("http://127.0.0.1:11434/v1".to_string(), None)
        .expect("create ollama OpenAI-compatible client");

    let max_chars = 12_000usize;
    let text_slice: String = page_text.chars().take(max_chars).collect();

    let sys = ChatMessage {
        role: Role::System,
        content: Some(
            "You extract weather info from web page text. Only use the provided text; if missing, say you cannot find it."
                .to_string(),
        ),
        name: None,
        tool_call_id: None,
        tool_calls: None,
    };
    let user = ChatMessage {
        role: Role::User,
        content: Some(format!(
            "We searched Baidu for: {query}\nURL: {page_url}\n\nPage text:\n{text_slice}\n\nTask: Return today's weather info in Chinese. Keep it short."
        )),
        name: None,
        tool_call_id: None,
        tool_calls: None,
    };

    let res = client
        .chat_completions(ChatCompletionRequest {
            model,
            messages: vec![sys, user],
            tools: vec![],
            temperature: Some(0.0),
        })
        .await
        .expect("ollama chat_completions");

    let summary = res.content.unwrap_or_default();
    println!("[rexos][baidu_weather] query={query}");
    println!("[rexos][baidu_weather] url={page_url}");
    println!("[rexos][baidu_weather] summary={summary}");

    assert!(!summary.trim().is_empty(), "empty summary");
}
