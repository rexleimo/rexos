use clap::Parser;
use std::path::PathBuf;

use rexos::{config::RexosConfig, memory::MemoryStore, paths::RexosPaths};

#[derive(Debug, Parser)]
#[command(name = "rexos")]
#[command(about = "RexOS: long-running agent operating system", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    /// Initialize ~/.rexos (config + database)
    Init,
    /// Run an agent session (LLM + tools + memory)
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },
    /// Long-running harness helpers (initializer + sessions)
    Harness {
        #[command(subcommand)]
        command: HarnessCommand,
    },
    /// Run RexOS daemon (HTTP API)
    Daemon {
        #[command(subcommand)]
        command: DaemonCommand,
    },
}

#[derive(Debug, clap::Subcommand)]
enum HarnessCommand {
    /// Initialize a workspace directory for long-running agent sessions
    Init {
        dir: PathBuf,
        /// Optional initializer prompt (generates a comprehensive features.json)
        #[arg(long)]
        prompt: Option<String>,
        /// Override session id (default: persisted per-workspace)
        #[arg(long)]
        session: Option<String>,
    },
    /// Run a harness session (preflight + agent run)
    Run {
        dir: PathBuf,
        /// User instruction for this session (if omitted, only runs preflight)
        #[arg(long)]
        prompt: Option<String>,
        /// Override session id (default: derived UUID per run)
        #[arg(long)]
        session: Option<String>,
        /// Max attempts when init.sh fails (default 3)
        #[arg(long, default_value_t = 3)]
        max_attempts: usize,
    },
}

#[derive(Debug, clap::Subcommand)]
enum AgentCommand {
    /// Run a single agent session in a workspace
    Run {
        /// Workspace root directory (tools are sandboxed to this path)
        #[arg(long)]
        workspace: PathBuf,
        /// User instruction for this run
        #[arg(long)]
        prompt: String,
        /// Optional system prompt (string)
        #[arg(long)]
        system: Option<String>,
        /// Optional session id (generated if omitted)
        #[arg(long)]
        session: Option<String>,
        /// Task kind for model routing
        #[arg(long, value_enum, default_value_t = AgentKind::Coding)]
        kind: AgentKind,
    },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum AgentKind {
    Planning,
    Coding,
    Summary,
}

impl From<AgentKind> for rexos::router::TaskKind {
    fn from(value: AgentKind) -> Self {
        match value {
            AgentKind::Planning => rexos::router::TaskKind::Planning,
            AgentKind::Coding => rexos::router::TaskKind::Coding,
            AgentKind::Summary => rexos::router::TaskKind::Summary,
        }
    }
}

#[derive(Debug, clap::Subcommand)]
enum DaemonCommand {
    /// Start the daemon HTTP server
    Start {
        /// Listen address, e.g. 127.0.0.1:8787
        #[arg(long, default_value = "127.0.0.1:8787")]
        addr: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init => {
            let paths = RexosPaths::discover()?;
            paths.ensure_dirs()?;
            RexosConfig::ensure_default(&paths)?;
            MemoryStore::open_or_create(&paths)?;
            println!("Initialized {}", paths.base_dir.display());
        }
        Command::Agent { command } => match command {
            AgentCommand::Run {
                workspace,
                prompt,
                system,
                session,
                kind,
            } => {
                let paths = RexosPaths::discover()?;
                paths.ensure_dirs()?;
                RexosConfig::ensure_default(&paths)?;
                let cfg = RexosConfig::load(&paths)?;

                let memory = MemoryStore::open_or_create(&paths)?;
                let llms = rexos::llm::registry::LlmRegistry::from_config(&cfg)?;
                let router = rexos::router::ModelRouter::new(cfg.router);
                let agent = rexos::agent::AgentRuntime::new(memory, llms, router);

                let session_id = session.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                let out = agent
                    .run_session(
                        workspace,
                        &session_id,
                        system.as_deref(),
                        &prompt,
                        kind.into(),
                    )
                    .await?;
                println!("{out}");
                eprintln!("[rexos] session_id={session_id}");
            }
        },
        Command::Harness { command } => match command {
            HarnessCommand::Init { dir, prompt, session } => {
                if prompt.is_none() {
                    rexos::harness::init_workspace(&dir)?;
                    println!("Harness initialized in {}", dir.display());
                    return Ok(());
                }

                match rexos::harness::init_workspace(&dir) {
                    Ok(()) => {}
                    Err(e) => {
                        let msg = e.to_string();
                        if !msg.contains("already initialized") {
                            return Err(e);
                        }
                    }
                }

                let session_id = match session {
                    Some(s) => s,
                    None => rexos::harness::resolve_session_id(&dir)?,
                };

                let prompt = prompt.expect("checked above");

                let paths = RexosPaths::discover()?;
                paths.ensure_dirs()?;
                RexosConfig::ensure_default(&paths)?;
                let cfg = RexosConfig::load(&paths)?;

                let memory = MemoryStore::open_or_create(&paths)?;
                let llms = rexos::llm::registry::LlmRegistry::from_config(&cfg)?;
                let router = rexos::router::ModelRouter::new(cfg.router);
                let agent = rexos::agent::AgentRuntime::new(memory, llms, router);

                rexos::harness::bootstrap_with_prompt(&agent, &dir, &session_id, &prompt).await?;

                println!("Harness bootstrapped in {}", dir.display());
                eprintln!("[rexos] session_id={session_id}");
            }
            HarnessCommand::Run {
                dir,
                prompt,
                session,
                max_attempts,
            } => {
                if prompt.is_none() {
                    rexos::harness::preflight(&dir)?;
                    return Ok(());
                }

                let session_id = match session {
                    Some(s) => s,
                    None => rexos::harness::resolve_session_id(&dir)?,
                };

                let prompt = prompt.expect("checked above");

                let paths = RexosPaths::discover()?;
                paths.ensure_dirs()?;
                RexosConfig::ensure_default(&paths)?;
                let cfg = RexosConfig::load(&paths)?;

                let memory = MemoryStore::open_or_create(&paths)?;
                let llms = rexos::llm::registry::LlmRegistry::from_config(&cfg)?;
                let router = rexos::router::ModelRouter::new(cfg.router);
                let agent = rexos::agent::AgentRuntime::new(memory, llms, router);

                let out =
                    rexos::harness::run_harness(&agent, &dir, &session_id, &prompt, max_attempts)
                        .await?;
                println!("{out}");
                eprintln!("[rexos] session_id={session_id}");
            }
        },
        Command::Daemon { command } => match command {
            DaemonCommand::Start { addr } => {
                let addr = addr.parse()?;
                rexos::daemon::serve(addr).await?;
            }
        },
    }

    Ok(())
}
