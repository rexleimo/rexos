use std::collections::BTreeMap;

#[tokio::test]
#[ignore]
async fn ollama_agent_shell_tool_smoke_creates_file() {
    let model = std::env::var("REXOS_OLLAMA_MODEL").unwrap_or_else(|_| "qwen3:4b".to_string());
    let base_url = std::env::var("REXOS_OLLAMA_BASE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:11434/v1".to_string());

    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();

    let home = tmp.path().join("home");
    let paths = rexos::paths::RexosPaths {
        base_dir: home.join(".rexos"),
    };
    paths.ensure_dirs().unwrap();

    let memory = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();

    let mut providers = BTreeMap::new();
    providers.insert(
        "ollama".to_string(),
        rexos::config::ProviderConfig {
            kind: rexos::config::ProviderKind::OpenAiCompatible,
            base_url,
            api_key_env: "".to_string(),
            default_model: model,
        },
    );

    let cfg = rexos::config::RexosConfig {
        llm: rexos::config::LlmConfig::default(),
        providers,
        router: rexos::config::RouterConfig::default(),
    };
    let llms = rexos::llm::registry::LlmRegistry::from_config(&cfg).unwrap();
    let router = rexos::router::ModelRouter::new(rexos::config::RouterConfig {
        planning: rexos::config::RouteConfig {
            provider: "ollama".to_string(),
            model: "default".to_string(),
        },
        coding: rexos::config::RouteConfig {
            provider: "ollama".to_string(),
            model: "default".to_string(),
        },
        summary: rexos::config::RouteConfig {
            provider: "ollama".to_string(),
            model: "default".to_string(),
        },
    });

    let agent = rexos::agent::AgentRuntime::new(memory, llms, router);
    let session_id = "ollama-shell-smoke";
    agent
        .set_session_allowed_tools(session_id, vec!["shell".to_string()])
        .unwrap();
    let shell_command = if cfg!(windows) {
        "New-Item -ItemType Directory -Path notes -Force | Out-Null; Set-Content -Path notes/shell-output.txt -Value shell-ok -NoNewline"
    } else {
        "mkdir -p notes && printf 'shell-ok' > notes/shell-output.txt"
    };

    let prompt = format!(
        "Use the shell tool to run this exact command: {shell_command}. \
After the tool returns, respond with DONE."
    );

    let out = agent
        .run_session(
            workspace.clone(),
            session_id,
            Some(
                "You are a strict tool-using assistant. \
Do not claim command execution unless you actually call the shell tool.",
            ),
            &prompt,
            rexos::router::TaskKind::Coding,
        )
        .await
        .unwrap();

    let written = std::fs::read_to_string(workspace.join("notes/shell-output.txt")).unwrap();
    assert_eq!(written, "shell-ok");
    assert!(!out.trim().is_empty());

    let memory = rexos::memory::MemoryStore::open_or_create(&paths).unwrap();
    let messages = memory.list_chat_messages(session_id).unwrap();
    let used_shell_tool = messages.iter().any(|m| {
        m.role == rexos::llm::openai_compat::Role::Tool && m.name.as_deref() == Some("shell")
    });
    assert!(used_shell_tool, "assistant did not call shell tool");
}
