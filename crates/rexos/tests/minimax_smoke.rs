use rexos::llm::driver::LlmDriver;

#[tokio::test]
#[ignore]
async fn minimax_native_smoke() {
    let api_key = std::env::var("MINIMAX_API_KEY")
        .or_else(|_| std::env::var("REXOS_MINIMAX_API_KEY"))
        .expect("set MINIMAX_API_KEY (or REXOS_MINIMAX_API_KEY) to run this test");

    let base_url = std::env::var("REXOS_MINIMAX_BASE_URL")
        .unwrap_or_else(|_| "https://api.minimax.chat/v1".to_string());
    let model = std::env::var("REXOS_MINIMAX_MODEL").unwrap_or_else(|_| "MiniMax-M2.5".to_string());

    let driver = rexos::llm::minimax::MiniMaxDriver::new(base_url, Some(api_key)).unwrap();

    let msg = driver
        .chat(rexos::llm::openai_compat::ChatCompletionRequest {
            model,
            messages: vec![rexos::llm::openai_compat::ChatMessage {
                role: rexos::llm::openai_compat::Role::User,
                content: Some("Reply with the single word: OK".to_string()),
                name: None,
                tool_call_id: None,
                tool_calls: None,
            }],
            tools: vec![],
            temperature: Some(0.0),
        })
        .await
        .unwrap();

    let content = msg.content.unwrap_or_default();
    assert!(!content.trim().is_empty());
}
