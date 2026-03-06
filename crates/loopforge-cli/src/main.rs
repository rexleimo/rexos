use anyhow::Context;
use clap::Parser;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;
use std::time::{SystemTime, UNIX_EPOCH};

use rexos::{
    config::{ProviderKind, RexosConfig},
    memory::MemoryStore,
    paths::RexosPaths,
};

mod doctor;
mod skills;

#[derive(Debug, Clone, serde::Serialize)]
struct ConfigValidationReport {
    valid: bool,
    config_path: String,
    errors: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct ReleaseCheckItem {
    id: String,
    ok: bool,
    message: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct ReleaseCheckReport {
    ok: bool,
    tag: String,
    checks: Vec<ReleaseCheckItem>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
struct OnboardMetrics {
    attempted_first_task: u64,
    first_task_success: u64,
    first_task_failed: u64,
    failure_by_category: BTreeMap<String, u64>,
    updated_at_ms: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
struct OnboardEvent {
    ts_ms: i64,
    workspace: String,
    session_id: String,
    outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure_category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum OnboardStarter {
    Hello,
    WorkspaceBrief,
    RepoOnboarding,
}

impl OnboardStarter {
    fn as_str(self) -> &'static str {
        match self {
            OnboardStarter::Hello => "hello",
            OnboardStarter::WorkspaceBrief => "workspace-brief",
            OnboardStarter::RepoOnboarding => "repo-onboarding",
        }
    }

    fn default_prompt(self) -> &'static str {
        match self {
            OnboardStarter::Hello => "Create hello.txt with the word hi",
            OnboardStarter::WorkspaceBrief => {
                "Create notes/workspace-brief.md with: what this workspace is for, 3 risks, and 3 next actions."
            }
            OnboardStarter::RepoOnboarding => {
                "Read README.md plus the most important project metadata files you can find. Create notes/repo-onboarding.md with: project purpose, how to run it, what to verify first, and 3 next actions."
            }
        }
    }

    fn description(self) -> &'static str {
        match self {
            OnboardStarter::Hello => "Minimal smoke check that proves file creation works.",
            OnboardStarter::WorkspaceBrief => {
                "Create a short workspace brief with risks and next actions."
            }
            OnboardStarter::RepoOnboarding => {
                "Read the repo and produce an onboarding brief for real project work."
            }
        }
    }

    fn expected_artifact(self) -> &'static str {
        match self {
            OnboardStarter::Hello => "hello.txt",
            OnboardStarter::WorkspaceBrief => "notes/workspace-brief.md",
            OnboardStarter::RepoOnboarding => "notes/repo-onboarding.md",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct OnboardStarterSuggestion {
    starter: String,
    description: String,
    command: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct OnboardTaskReport {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure_category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct OnboardReportArtifact {
    generated_at_ms: i64,
    workspace: String,
    config_path: String,
    config_valid: bool,
    starter: String,
    effective_prompt: String,
    doctor_summary: doctor::DoctorSummary,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    doctor_next_actions: Vec<String>,
    task: OnboardTaskReport,
    recommended_next_command: String,
    starter_suggestions: Vec<OnboardStarterSuggestion>,
}

#[derive(Debug, Parser)]
#[command(name = "loopforge")]
#[command(
    about = "LoopForge: long-running agent operating system",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    /// Initialize ~/.loopforge (config + database)
    Init,
    /// One-command onboarding check (init + config + doctor + optional first task)
    Onboard {
        /// Workspace directory for the first verification run
        #[arg(long, default_value = "loopforge-onboard-demo")]
        workspace: PathBuf,
        /// Optional explicit prompt for the first verification run
        #[arg(long)]
        prompt: Option<String>,
        /// Starter profile used when `--prompt` is not provided
        #[arg(long, value_enum, default_value_t = OnboardStarter::Hello)]
        starter: OnboardStarter,
        /// Skip running the first agent task and only run setup checks
        #[arg(long)]
        skip_agent: bool,
        /// Timeout for doctor probes (milliseconds)
        #[arg(long, default_value_t = 1500)]
        timeout_ms: u64,
    },
    /// Diagnose common setup issues (config, providers, browser, tooling)
    Doctor {
        /// Print JSON output (machine-readable)
        #[arg(long)]
        json: bool,
        /// Exit non-zero if any warnings are detected
        #[arg(long)]
        strict: bool,
        /// Timeout for network probes (milliseconds)
        #[arg(long, default_value_t = 1500)]
        timeout_ms: u64,
    },
    /// Run an agent session (LLM + tools + memory)
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },
    /// Outbound channels (outbox + dispatcher)
    Channel {
        #[command(subcommand)]
        command: ChannelCommand,
    },
    /// ACP event/checkpoint inspection helpers
    Acp {
        #[command(subcommand)]
        command: AcpCommand,
    },
    /// Config helpers
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Skills discovery, doctor and execution helpers
    Skills {
        #[command(subcommand)]
        command: SkillsCommand,
    },
    /// Long-running harness helpers (initializer + sessions)
    Harness {
        #[command(subcommand)]
        command: HarnessCommand,
    },
    /// Run LoopForge daemon (HTTP API)
    Daemon {
        #[command(subcommand)]
        command: DaemonCommand,
    },
    /// Release assistants (metadata + preflight checks)
    Release {
        #[command(subcommand)]
        command: ReleaseCommand,
    },
}

#[derive(Debug, clap::Subcommand)]
enum ConfigCommand {
    /// Validate ~/.loopforge/config.toml and exit non-zero when invalid
    Validate {
        /// Print JSON output (machine-readable)
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, clap::Subcommand)]
enum SkillsCommand {
    /// List discovered skills (workspace + ~/.codex/skills)
    List {
        /// Workspace root directory
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        /// Print JSON output (machine-readable)
        #[arg(long)]
        json: bool,
    },
    /// Show one skill's resolved metadata
    Show {
        /// Skill name
        name: String,
        /// Workspace root directory
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        /// Print JSON output (machine-readable)
        #[arg(long)]
        json: bool,
    },
    /// Diagnose skill manifest and entry issues
    Doctor {
        /// Workspace root directory
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        /// Print JSON output (machine-readable)
        #[arg(long)]
        json: bool,
        /// Exit non-zero on warnings too
        #[arg(long)]
        strict: bool,
    },
    /// Execute one skill with real runtime tools and model routing
    Run {
        /// Skill name
        name: String,
        /// Workspace root directory
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        /// Input payload passed to the skill
        #[arg(long)]
        input: String,
        /// Optional session id (generated per-workspace if omitted)
        #[arg(long)]
        session: Option<String>,
        /// Task kind for model routing
        #[arg(long, value_enum, default_value_t = AgentKind::Coding)]
        kind: AgentKind,
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
        /// Comma-separated allowed tool names for this session (session-level whitelist)
        #[arg(long, value_delimiter = ',')]
        allowed_tools: Vec<String>,
    },
}

#[derive(Debug, clap::Subcommand)]
enum ChannelCommand {
    /// Drain queued outbox messages once
    Drain {
        /// Max messages to attempt in one run
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// Run a long-lived worker that periodically drains the outbox
    Worker {
        /// Seconds between drain attempts
        #[arg(long, default_value_t = 5)]
        interval_secs: u64,
        /// Max messages to attempt per drain cycle
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
}

#[derive(Debug, clap::Subcommand)]
enum AcpCommand {
    /// List recent ACP events
    Events {
        /// Optional session id filter
        #[arg(long)]
        session: Option<String>,
        /// Max events to print
        #[arg(long, default_value_t = 100)]
        limit: usize,
        /// Print JSON output (machine-readable)
        #[arg(long)]
        json: bool,
    },
    /// Show ACP delivery checkpoints for one session
    Checkpoints {
        /// Session id
        #[arg(long)]
        session: String,
        /// Print JSON output (machine-readable)
        #[arg(long)]
        json: bool,
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

#[derive(Debug, clap::Subcommand)]
enum ReleaseCommand {
    /// Check release metadata and preflight conditions
    Check {
        /// Release tag, e.g. v0.1.0 (defaults to v<workspace version>)
        #[arg(long)]
        tag: Option<String>,
        /// Repository root to check (default: current directory)
        #[arg(long, default_value = ".")]
        repo_root: PathBuf,
        /// Run `cargo test --workspace --locked` as part of preflight
        #[arg(long)]
        run_tests: bool,
        /// Print JSON output (machine-readable)
        #[arg(long)]
        json: bool,
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
        Command::Onboard {
            workspace,
            prompt,
            starter,
            skip_agent,
            timeout_ms,
        } => {
            let paths = RexosPaths::discover()?;
            paths.ensure_dirs()?;
            RexosConfig::ensure_default(&paths)?;
            MemoryStore::open_or_create(&paths)?;
            println!("Initialized {}", paths.base_dir.display());

            std::fs::create_dir_all(&workspace)
                .with_context(|| format!("create workspace: {}", workspace.display()))?;
            println!("workspace ready: {}", workspace.display());

            let effective_prompt = resolve_onboard_prompt(prompt.as_deref(), starter);
            let config_report = validate_config(&paths);
            if !config_report.valid {
                println!("config invalid: {}", config_report.config_path);
                for err in &config_report.errors {
                    println!("- {err}");
                }
                let report = OnboardReportArtifact {
                    generated_at_ms: now_ms(),
                    workspace: workspace.display().to_string(),
                    config_path: config_report.config_path.clone(),
                    config_valid: false,
                    starter: starter.as_str().to_string(),
                    effective_prompt: effective_prompt.clone(),
                    doctor_summary: doctor::DoctorSummary {
                        ok: 0,
                        warn: 0,
                        error: 1,
                    },
                    doctor_next_actions: vec![format!(
                        "Fix `{}` and rerun `loopforge config validate`.",
                        config_report.config_path
                    )],
                    task: OnboardTaskReport {
                        status: "blocked".to_string(),
                        session_id: None,
                        failure_category: Some("config_invalid".to_string()),
                        error: Some(config_report.errors.join("; ")),
                    },
                    recommended_next_command: "loopforge config validate".to_string(),
                    starter_suggestions: build_onboard_starter_suggestions(&workspace),
                };
                let (json_path, md_path) = write_onboard_report(&workspace, &report)?;
                print_onboard_report_summary(&report, &json_path, &md_path);
                std::process::exit(1);
            }
            println!("config valid: {}", config_report.config_path);

            let doctor_report = doctor::run_doctor(doctor::DoctorOptions {
                paths: paths.clone(),
                timeout: std::time::Duration::from_millis(timeout_ms),
            })
            .await?;
            println!("{}", doctor_report.to_text());
            let blocking_errors: Vec<&doctor::DoctorCheck> = doctor_report
                .checks
                .iter()
                .filter(|c| is_onboard_blocking_doctor_error(c))
                .collect();
            if !blocking_errors.is_empty() {
                eprintln!("onboard blocked by critical setup errors:");
                for c in &blocking_errors {
                    eprintln!("- {}: {}", c.id, c.message);
                }
                let report = OnboardReportArtifact {
                    generated_at_ms: now_ms(),
                    workspace: workspace.display().to_string(),
                    config_path: config_report.config_path.clone(),
                    config_valid: true,
                    starter: starter.as_str().to_string(),
                    effective_prompt: effective_prompt.clone(),
                    doctor_summary: doctor_report.summary.clone(),
                    doctor_next_actions: doctor_report.next_actions.clone(),
                    task: OnboardTaskReport {
                        status: "blocked".to_string(),
                        session_id: None,
                        failure_category: Some("doctor_blocked".to_string()),
                        error: Some(
                            blocking_errors
                                .iter()
                                .map(|check| format!("{}: {}", check.id, check.message))
                                .collect::<Vec<_>>()
                                .join("; "),
                        ),
                    },
                    recommended_next_command: "loopforge doctor".to_string(),
                    starter_suggestions: build_onboard_starter_suggestions(&workspace),
                };
                let (json_path, md_path) = write_onboard_report(&workspace, &report)?;
                print_onboard_report_summary(&report, &json_path, &md_path);
                std::process::exit(1);
            }
            let non_blocking_errors = doctor_report
                .checks
                .iter()
                .filter(|c| c.status == doctor::CheckStatus::Error)
                .count()
                .saturating_sub(blocking_errors.len());
            if non_blocking_errors > 0 {
                eprintln!(
                    "onboard: continuing despite {} non-blocking doctor error(s)",
                    non_blocking_errors
                );
            }

            if skip_agent {
                let report = OnboardReportArtifact {
                    generated_at_ms: now_ms(),
                    workspace: workspace.display().to_string(),
                    config_path: config_report.config_path.clone(),
                    config_valid: true,
                    starter: starter.as_str().to_string(),
                    effective_prompt: effective_prompt.clone(),
                    doctor_summary: doctor_report.summary.clone(),
                    doctor_next_actions: doctor_report.next_actions.clone(),
                    task: OnboardTaskReport {
                        status: "skipped".to_string(),
                        session_id: None,
                        failure_category: None,
                        error: None,
                    },
                    recommended_next_command: format!(
                        "loopforge onboard --workspace {} --starter {}",
                        shell_quote(&workspace.display().to_string()),
                        starter.as_str()
                    ),
                    starter_suggestions: build_onboard_starter_suggestions(&workspace),
                };
                let (json_path, md_path) = write_onboard_report(&workspace, &report)?;
                print_onboard_report_summary(&report, &json_path, &md_path);
                println!("onboard done (skipped first agent run)");
                return Ok(());
            }

            let cfg = RexosConfig::load(&paths)?;
            let mut cfg = cfg;
            if cfg.router.coding.provider.trim() == "ollama" {
                let maybe_ollama = cfg.providers.get("ollama").cloned();
                if let Some(ollama) = maybe_ollama {
                    if ollama.kind == ProviderKind::OpenAiCompatible {
                        if let Ok(models) =
                            fetch_openai_compat_models(&ollama.base_url, timeout_ms).await
                        {
                            if let Some(selected) =
                                select_onboard_model(&ollama.default_model, &models)
                            {
                                if selected != ollama.default_model {
                                    if let Some(p) = cfg.providers.get_mut("ollama") {
                                        p.default_model = selected.clone();
                                    }
                                    println!(
                                        "onboard: ollama default model '{}' not available, using '{}'",
                                        ollama.default_model, selected
                                    );
                                }
                            }
                        }
                    }
                }
            }

            let memory = MemoryStore::open_or_create(&paths)?;
            let llms = rexos::llm::registry::LlmRegistry::from_config(&cfg)?;
            let router = rexos::router::ModelRouter::new(cfg.router);
            let agent = rexos::agent::AgentRuntime::new(memory, llms, router);

            let session_id = rexos::harness::resolve_session_id(&workspace)?;
            let out = match agent
                .run_session(
                    workspace.clone(),
                    &session_id,
                    None,
                    &effective_prompt,
                    rexos::router::TaskKind::Coding,
                )
                .await
            {
                Ok(out) => out,
                Err(e) => {
                    let err_msg = e.to_string();
                    let failure_category = classify_onboard_failure(&err_msg);
                    match record_onboard_attempt(
                        &paths,
                        &workspace,
                        &session_id,
                        false,
                        Some(&failure_category),
                        Some(&err_msg),
                    ) {
                        Ok(metrics) => {
                            eprintln!(
                                "onboard metrics: success_rate={}/{}",
                                metrics.first_task_success, metrics.attempted_first_task
                            );
                            eprintln!(
                                "onboard metrics path: {}",
                                paths.base_dir.join("onboard-metrics.json").display()
                            );
                            eprintln!(
                                "onboard events path: {}",
                                paths.base_dir.join("onboard-events.jsonl").display()
                            );
                        }
                        Err(log_err) => {
                            eprintln!("onboard: failed to persist metrics: {log_err}");
                        }
                    }
                    let report = OnboardReportArtifact {
                        generated_at_ms: now_ms(),
                        workspace: workspace.display().to_string(),
                        config_path: config_report.config_path.clone(),
                        config_valid: true,
                        starter: starter.as_str().to_string(),
                        effective_prompt: effective_prompt.clone(),
                        doctor_summary: doctor_report.summary.clone(),
                        doctor_next_actions: doctor_report.next_actions.clone(),
                        task: OnboardTaskReport {
                            status: "failed".to_string(),
                            session_id: Some(session_id.clone()),
                            failure_category: Some(failure_category.clone()),
                            error: Some(err_msg.clone()),
                        },
                        recommended_next_command: recommended_next_command(&workspace, false),
                        starter_suggestions: build_onboard_starter_suggestions(&workspace),
                    };
                    let (json_path, md_path) = write_onboard_report(&workspace, &report)?;
                    print_onboard_report_summary(&report, &json_path, &md_path);
                    eprintln!("onboard: first agent run failed: {e}");
                    eprintln!(
                        "hint: run `ollama list` and set [providers.ollama].default_model in ~/.loopforge/config.toml to an available chat model"
                    );
                    return Err(e);
                }
            };
            println!("{out}");
            eprintln!("[loopforge] session_id={session_id}");

            if let Err(err) = verify_onboard_artifact(&workspace, prompt.as_deref(), starter) {
                let err_msg = err.to_string();
                let failure_category = "expected_artifact_missing".to_string();
                match record_onboard_attempt(
                    &paths,
                    &workspace,
                    &session_id,
                    false,
                    Some(&failure_category),
                    Some(&err_msg),
                ) {
                    Ok(metrics) => {
                        eprintln!(
                            "onboard metrics: success_rate={}/{}",
                            metrics.first_task_success, metrics.attempted_first_task
                        );
                        eprintln!(
                            "onboard metrics path: {}",
                            paths.base_dir.join("onboard-metrics.json").display()
                        );
                        eprintln!(
                            "onboard events path: {}",
                            paths.base_dir.join("onboard-events.jsonl").display()
                        );
                    }
                    Err(log_err) => {
                        eprintln!("onboard: failed to persist metrics: {log_err}");
                    }
                }
                let report = OnboardReportArtifact {
                    generated_at_ms: now_ms(),
                    workspace: workspace.display().to_string(),
                    config_path: config_report.config_path.clone(),
                    config_valid: true,
                    starter: starter.as_str().to_string(),
                    effective_prompt: effective_prompt.clone(),
                    doctor_summary: doctor_report.summary.clone(),
                    doctor_next_actions: doctor_report.next_actions.clone(),
                    task: OnboardTaskReport {
                        status: "failed".to_string(),
                        session_id: Some(session_id.clone()),
                        failure_category: Some(failure_category.clone()),
                        error: Some(err_msg.clone()),
                    },
                    recommended_next_command: recommended_retry_command(
                        &workspace,
                        &session_id,
                        &effective_prompt,
                    ),
                    starter_suggestions: build_onboard_starter_suggestions(&workspace),
                };
                let (json_path, md_path) = write_onboard_report(&workspace, &report)?;
                print_onboard_report_summary(&report, &json_path, &md_path);
                eprintln!(
                    "onboard: first agent run finished without the expected starter artifact"
                );
                return Err(err);
            }

            match record_onboard_attempt(&paths, &workspace, &session_id, true, None, None) {
                Ok(metrics) => {
                    println!(
                        "onboard metrics: success_rate={}/{}",
                        metrics.first_task_success, metrics.attempted_first_task
                    );
                }
                Err(log_err) => {
                    eprintln!("onboard: failed to persist metrics: {log_err}");
                }
            }
            let report = OnboardReportArtifact {
                generated_at_ms: now_ms(),
                workspace: workspace.display().to_string(),
                config_path: config_report.config_path.clone(),
                config_valid: true,
                starter: starter.as_str().to_string(),
                effective_prompt: effective_prompt.clone(),
                doctor_summary: doctor_report.summary.clone(),
                doctor_next_actions: doctor_report.next_actions.clone(),
                task: OnboardTaskReport {
                    status: "succeeded".to_string(),
                    session_id: Some(session_id.clone()),
                    failure_category: None,
                    error: None,
                },
                recommended_next_command: recommended_next_command(&workspace, true),
                starter_suggestions: build_onboard_starter_suggestions(&workspace),
            };
            let (json_path, md_path) = write_onboard_report(&workspace, &report)?;
            print_onboard_report_summary(&report, &json_path, &md_path);
            println!("onboard done (first agent run completed)");
        }
        Command::Doctor {
            json,
            strict,
            timeout_ms,
        } => {
            let paths = RexosPaths::discover()?;
            let report = doctor::run_doctor(doctor::DoctorOptions {
                paths,
                timeout: std::time::Duration::from_millis(timeout_ms),
            })
            .await?;

            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{}", report.to_text());
            }

            let code = report.exit_code(strict);
            if code != 0 {
                std::process::exit(code);
            }
        }
        Command::Agent { command } => match command {
            AgentCommand::Run {
                workspace,
                prompt,
                system,
                session,
                kind,
                allowed_tools,
            } => {
                let paths = RexosPaths::discover()?;
                paths.ensure_dirs()?;
                RexosConfig::ensure_default(&paths)?;
                let cfg = RexosConfig::load(&paths)?;

                let memory = MemoryStore::open_or_create(&paths)?;
                let llms = rexos::llm::registry::LlmRegistry::from_config(&cfg)?;
                let router = rexos::router::ModelRouter::new(cfg.router);
                let agent = rexos::agent::AgentRuntime::new(memory, llms, router);

                let session_id = match session {
                    Some(id) => id,
                    None => rexos::harness::resolve_session_id(&workspace)?,
                };
                if !allowed_tools.is_empty() {
                    agent.set_session_allowed_tools(&session_id, allowed_tools)?;
                }
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
                eprintln!("[loopforge] session_id={session_id}");
            }
        },
        Command::Channel { command } => match command {
            ChannelCommand::Drain { limit } => {
                let paths = RexosPaths::discover()?;
                paths.ensure_dirs()?;
                RexosConfig::ensure_default(&paths)?;
                MemoryStore::open_or_create(&paths)?;

                let dispatcher =
                    rexos::agent::OutboxDispatcher::new(MemoryStore::open_or_create(&paths)?)?;
                let summary = dispatcher.drain_once(limit).await?;
                println!("drain: sent={} failed={}", summary.sent, summary.failed);
            }
            ChannelCommand::Worker {
                interval_secs,
                limit,
            } => {
                let paths = RexosPaths::discover()?;
                paths.ensure_dirs()?;
                RexosConfig::ensure_default(&paths)?;
                MemoryStore::open_or_create(&paths)?;

                let dispatcher =
                    rexos::agent::OutboxDispatcher::new(MemoryStore::open_or_create(&paths)?)?;

                loop {
                    let summary = dispatcher.drain_once(limit).await?;
                    println!("drain: sent={} failed={}", summary.sent, summary.failed);
                    tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;
                }
            }
        },
        Command::Acp { command } => match command {
            AcpCommand::Events {
                session,
                limit,
                json,
            } => {
                let paths = RexosPaths::discover()?;
                paths.ensure_dirs()?;
                RexosConfig::ensure_default(&paths)?;
                let memory = MemoryStore::open_or_create(&paths)?;

                let events = load_acp_events(&memory, session.as_deref(), limit)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&events)?);
                } else {
                    for ev in events {
                        let session = ev.get("session_id").and_then(|v| v.as_str()).unwrap_or("-");
                        let event_type = ev
                            .get("event_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        let created_at = ev.get("created_at").and_then(|v| v.as_i64()).unwrap_or(0);
                        println!("[{created_at}] session={session} type={event_type}");
                    }
                }
            }
            AcpCommand::Checkpoints { session, json } => {
                let paths = RexosPaths::discover()?;
                paths.ensure_dirs()?;
                RexosConfig::ensure_default(&paths)?;
                let memory = MemoryStore::open_or_create(&paths)?;

                let checkpoints = load_acp_checkpoints(&memory, &session)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&checkpoints)?);
                } else if checkpoints.is_empty() {
                    println!("no checkpoints for session {}", session);
                } else {
                    for cp in checkpoints {
                        let channel = cp.get("channel").and_then(|v| v.as_str()).unwrap_or("-");
                        let cursor = cp.get("cursor").and_then(|v| v.as_str()).unwrap_or("-");
                        let updated_at = cp.get("updated_at").and_then(|v| v.as_i64()).unwrap_or(0);
                        println!("[{updated_at}] channel={channel} cursor={cursor}");
                    }
                }
            }
        },
        Command::Config { command } => match command {
            ConfigCommand::Validate { json } => {
                let paths = RexosPaths::discover()?;
                let report = validate_config(&paths);
                if json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else if report.valid {
                    println!("config valid: {}", report.config_path);
                } else {
                    println!("config invalid: {}", report.config_path);
                    for err in &report.errors {
                        println!("- {err}");
                    }
                }

                if !report.valid {
                    std::process::exit(1);
                }
            }
        },
        Command::Skills { command } => match command {
            SkillsCommand::List { workspace, json } => {
                let list = skills::list_skills(&workspace)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&list)?);
                } else if list.is_empty() {
                    println!("no skills discovered");
                } else {
                    for item in list {
                        println!(
                            "{}  v{}  source={}  entry={}",
                            item.name, item.version, item.source, item.entry_path
                        );
                    }
                }
            }
            SkillsCommand::Show {
                name,
                workspace,
                json,
            } => {
                let skill = skills::find_skill(&workspace, &name)?;
                let item = serde_json::json!({
                    "name": skill.name,
                    "version": skill.manifest.version.to_string(),
                    "source": skills::source_name(skill.source),
                    "root_dir": skill.root_dir,
                    "manifest_path": skill.manifest_path,
                    "entry": skill.manifest.entry,
                    "permissions": skill.manifest.permissions,
                    "dependencies": skill
                        .manifest
                        .dependencies
                        .iter()
                        .map(|d| serde_json::json!({
                            "name": d.name,
                            "version_req": d.version_req.to_string(),
                        }))
                        .collect::<Vec<_>>(),
                });
                if json {
                    println!("{}", serde_json::to_string_pretty(&item)?);
                } else {
                    println!("{}", serde_json::to_string_pretty(&item)?);
                }
            }
            SkillsCommand::Doctor {
                workspace,
                json,
                strict,
            } => {
                let report = skills::doctor(&workspace)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    println!("discovered_skills: {}", report.discovered_count);
                    if report.issues.is_empty() {
                        println!("doctor: ok");
                    } else {
                        for issue in &report.issues {
                            let level = match issue.level {
                                skills::SkillsDoctorLevel::Warn => "warn",
                                skills::SkillsDoctorLevel::Error => "error",
                            };
                            if let Some(path) = &issue.path {
                                println!("[{level}] {}: {} ({path})", issue.id, issue.message);
                            } else {
                                println!("[{level}] {}: {}", issue.id, issue.message);
                            }
                        }
                    }
                }

                let has_error = report
                    .issues
                    .iter()
                    .any(|i| matches!(i.level, skills::SkillsDoctorLevel::Error));
                let has_warn = report
                    .issues
                    .iter()
                    .any(|i| matches!(i.level, skills::SkillsDoctorLevel::Warn));
                if has_error || (strict && has_warn) {
                    std::process::exit(1);
                }
            }
            SkillsCommand::Run {
                name,
                workspace,
                input,
                session,
                kind,
            } => {
                let paths = RexosPaths::discover()?;
                paths.ensure_dirs()?;
                RexosConfig::ensure_default(&paths)?;
                let cfg = RexosConfig::load(&paths)?;
                let skills_cfg = RexosConfig::load_skills_config(&paths).unwrap_or_default();

                std::fs::create_dir_all(&workspace)
                    .with_context(|| format!("create workspace: {}", workspace.display()))?;

                let skill = skills::find_skill(&workspace, &name)?;
                let skill_entry = skills::read_skill_entry(&skill)?;

                let memory = MemoryStore::open_or_create(&paths)?;
                let llms = rexos::llm::registry::LlmRegistry::from_config(&cfg)?;
                let router = rexos::router::ModelRouter::new(cfg.router);
                let agent = rexos::agent::AgentRuntime::new(memory, llms, router);

                let session_id = match session {
                    Some(id) => id,
                    None => rexos::harness::resolve_session_id(&workspace)?,
                };
                let experimental_mode = skills_cfg.experimental;
                agent.set_session_skill_policy(
                    &session_id,
                    rexos::agent::SessionSkillPolicy {
                        allowlist: skills_cfg.allowlist,
                        require_approval: skills_cfg.require_approval,
                        auto_approve_readonly: skills_cfg.auto_approve_readonly,
                    },
                )?;
                if experimental_mode {
                    eprintln!("skills: experimental mode is enabled in config");
                }

                agent.record_skill_discovered(
                    &session_id,
                    &skill.name,
                    skills::source_name(skill.source),
                    &skill.manifest.version.to_string(),
                )?;
                agent.authorize_skill(&session_id, &skill.name, &skill.manifest.permissions)?;

                let allowed_tools = skills::permission_tools(&skill.manifest.permissions);
                if !allowed_tools.is_empty() {
                    agent.set_session_allowed_tools(&session_id, allowed_tools)?;
                }

                let system = format!(
                    "You are executing skill `{}` version {}.\\n\
Follow the skill instructions exactly.\\n\
If tool permissions are restricted, do not call tools outside the granted scope.\\n\\n\
--- SKILL INSTRUCTIONS START ---\\n{}\\n--- SKILL INSTRUCTIONS END ---",
                    skill.name, skill.manifest.version, skill_entry
                );

                let out = match agent
                    .run_session(workspace, &session_id, Some(&system), &input, kind.into())
                    .await
                {
                    Ok(out) => {
                        agent.record_skill_execution(
                            &session_id,
                            &skill.name,
                            &skill.manifest.permissions,
                            true,
                            None,
                        )?;
                        out
                    }
                    Err(e) => {
                        let err_text = e.to_string();
                        let _ = agent.record_skill_execution(
                            &session_id,
                            &skill.name,
                            &skill.manifest.permissions,
                            false,
                            Some(&err_text),
                        );
                        return Err(e);
                    }
                };

                println!("{out}");
                eprintln!("[loopforge] session_id={session_id}");
            }
        },
        Command::Harness { command } => match command {
            HarnessCommand::Init {
                dir,
                prompt,
                session,
            } => {
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
                eprintln!("[loopforge] session_id={session_id}");
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
                eprintln!("[loopforge] session_id={session_id}");
            }
        },
        Command::Daemon { command } => match command {
            DaemonCommand::Start { addr } => {
                let addr = addr.parse()?;
                rexos::daemon::serve(addr).await?;
            }
        },
        Command::Release { command } => match command {
            ReleaseCommand::Check {
                tag,
                repo_root,
                run_tests,
                json,
            } => {
                let report = run_release_check(&repo_root, tag.as_deref(), run_tests)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    println!("{}", format_release_check_report(&report));
                }
                if !report.ok {
                    std::process::exit(1);
                }
            }
        },
    }

    Ok(())
}

fn validate_config(paths: &RexosPaths) -> ConfigValidationReport {
    let config_path = paths.config_path();
    let display_path = config_path.display().to_string();
    let raw = match std::fs::read_to_string(&config_path) {
        Ok(raw) => raw,
        Err(e) => {
            return ConfigValidationReport {
                valid: false,
                config_path: display_path,
                errors: vec![format!("read config failed: {e}")],
            };
        }
    };

    let cfg: RexosConfig = match toml::from_str(&raw) {
        Ok(cfg) => cfg,
        Err(e) => {
            return ConfigValidationReport {
                valid: false,
                config_path: display_path,
                errors: vec![format!("parse config TOML failed: {e}")],
            };
        }
    };

    let mut errors = Vec::new();
    for (route_name, provider_name) in [
        ("planning", cfg.router.planning.provider.trim()),
        ("coding", cfg.router.coding.provider.trim()),
        ("summary", cfg.router.summary.provider.trim()),
    ] {
        if provider_name.is_empty() {
            errors.push(format!("router.{route_name}.provider is empty"));
            continue;
        }
        if !cfg.providers.contains_key(provider_name) {
            errors.push(format!(
                "router.{route_name}.provider references unknown provider '{provider_name}'"
            ));
        }
    }

    ConfigValidationReport {
        valid: errors.is_empty(),
        config_path: display_path,
        errors,
    }
}

fn select_onboard_model(preferred: &str, available: &[String]) -> Option<String> {
    if available.is_empty() {
        return None;
    }
    let preferred = preferred.trim();
    if !preferred.is_empty() {
        if let Some(hit) = available
            .iter()
            .find(|m| m.trim().eq_ignore_ascii_case(preferred))
        {
            return Some(hit.clone());
        }
    }

    if let Some(chat_like) = available.iter().find(|m| {
        let lower = m.to_ascii_lowercase();
        !lower.contains("embed")
    }) {
        return Some(chat_like.clone());
    }
    Some(available[0].clone())
}

fn is_onboard_blocking_doctor_error(check: &doctor::DoctorCheck) -> bool {
    if check.status != doctor::CheckStatus::Error {
        return false;
    }
    check.id == "config.parse" || check.id.starts_with("router.")
}

fn onboard_metrics_path(paths: &RexosPaths) -> PathBuf {
    paths.base_dir.join("onboard-metrics.json")
}

fn onboard_events_path(paths: &RexosPaths) -> PathBuf {
    paths.base_dir.join("onboard-events.jsonl")
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn classify_onboard_failure(err_msg: &str) -> String {
    let lower = err_msg.to_ascii_lowercase();

    let looks_like_model =
        lower.contains("model") && (lower.contains("not found") || lower.contains("unknown"));
    if looks_like_model || lower.contains("embedding-only") || lower.contains("no chat model") {
        return "model_unavailable".to_string();
    }

    let looks_like_connectivity = lower.contains("timed out")
        || lower.contains("connection refused")
        || lower.contains("failed to send request")
        || lower.contains("dns")
        || lower.contains("name or service not known")
        || lower.contains("http");
    if looks_like_connectivity {
        return "provider_unreachable".to_string();
    }

    if lower.contains("tool") {
        return "tool_runtime_error".to_string();
    }

    if lower.contains("sandbox") || lower.contains("permission denied") {
        return "sandbox_restriction".to_string();
    }

    "unknown".to_string()
}

fn load_onboard_metrics(paths: &RexosPaths) -> OnboardMetrics {
    let p = onboard_metrics_path(paths);
    match std::fs::read_to_string(&p) {
        Ok(raw) => serde_json::from_str::<OnboardMetrics>(&raw).unwrap_or_default(),
        Err(_) => OnboardMetrics::default(),
    }
}

fn save_onboard_metrics(paths: &RexosPaths, metrics: &OnboardMetrics) -> anyhow::Result<()> {
    let p = onboard_metrics_path(paths);
    let raw = serde_json::to_string_pretty(metrics)?;
    std::fs::write(&p, raw).with_context(|| format!("write {}", p.display()))?;
    Ok(())
}

fn append_onboard_event(paths: &RexosPaths, event: &OnboardEvent) -> anyhow::Result<()> {
    let p = onboard_events_path(paths);
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&p)
        .with_context(|| format!("open {}", p.display()))?;
    let line = serde_json::to_string(event)?;
    writeln!(f, "{line}").with_context(|| format!("append {}", p.display()))?;
    Ok(())
}

fn record_onboard_attempt(
    paths: &RexosPaths,
    workspace: &Path,
    session_id: &str,
    success: bool,
    failure_category: Option<&str>,
    error: Option<&str>,
) -> anyhow::Result<OnboardMetrics> {
    let mut metrics = load_onboard_metrics(paths);
    metrics.attempted_first_task += 1;
    if success {
        metrics.first_task_success += 1;
    } else {
        metrics.first_task_failed += 1;
        if let Some(category) = failure_category {
            let entry = metrics
                .failure_by_category
                .entry(category.to_string())
                .or_insert(0);
            *entry += 1;
        }
    }
    metrics.updated_at_ms = now_ms();
    save_onboard_metrics(paths, &metrics)?;

    let event = OnboardEvent {
        ts_ms: metrics.updated_at_ms,
        workspace: workspace.display().to_string(),
        session_id: session_id.to_string(),
        outcome: if success {
            "success".to_string()
        } else {
            "failed".to_string()
        },
        failure_category: failure_category.map(|s| s.to_string()),
        error: error.map(|s| s.to_string()),
    };
    append_onboard_event(paths, &event)?;

    Ok(metrics)
}

fn resolve_onboard_prompt(prompt: Option<&str>, starter: OnboardStarter) -> String {
    prompt
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .unwrap_or_else(|| starter.default_prompt().to_string())
}

fn starter_expected_artifact(
    prompt: Option<&str>,
    starter: OnboardStarter,
) -> Option<&'static str> {
    let has_explicit_prompt = prompt
        .map(str::trim)
        .map(|value| !value.is_empty())
        .unwrap_or(false);
    if has_explicit_prompt {
        None
    } else {
        Some(starter.expected_artifact())
    }
}

fn verify_onboard_artifact(
    workspace: &Path,
    prompt: Option<&str>,
    starter: OnboardStarter,
) -> anyhow::Result<Option<String>> {
    let Some(relative_path) = starter_expected_artifact(prompt, starter) else {
        return Ok(None);
    };

    let artifact_path = workspace.join(relative_path);
    if artifact_path.is_file() {
        return Ok(Some(relative_path.to_string()));
    }

    anyhow::bail!(
        "starter `{}` did not create expected artifact `{}` in `{}`",
        starter.as_str(),
        relative_path,
        workspace.display()
    );
}

fn recommended_retry_command(workspace: &Path, session_id: &str, prompt: &str) -> String {
    format!(
        "loopforge agent run --workspace {} --session {} --prompt {}",
        shell_quote(&workspace.display().to_string()),
        shell_quote(session_id),
        shell_quote(prompt)
    )
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn recommended_next_command(workspace: &Path, success: bool) -> String {
    if success {
        format!(
            "loopforge agent run --workspace {} --prompt {}",
            shell_quote(&workspace.display().to_string()),
            shell_quote("Continue from the current workspace and write notes/next-steps.md with 3 follow-up actions.")
        )
    } else {
        "loopforge doctor".to_string()
    }
}

fn build_onboard_starter_suggestions(workspace: &Path) -> Vec<OnboardStarterSuggestion> {
    [
        OnboardStarter::Hello,
        OnboardStarter::WorkspaceBrief,
        OnboardStarter::RepoOnboarding,
    ]
    .into_iter()
    .map(|starter| OnboardStarterSuggestion {
        starter: starter.as_str().to_string(),
        description: starter.description().to_string(),
        command: format!(
            "loopforge onboard --workspace {} --starter {}",
            shell_quote(&workspace.display().to_string()),
            starter.as_str()
        ),
    })
    .collect()
}

fn onboard_report_paths(workspace: &Path) -> (PathBuf, PathBuf) {
    let dir = workspace.join(".loopforge");
    (
        dir.join("onboard-report.json"),
        dir.join("onboard-report.md"),
    )
}

fn render_onboard_report_markdown(report: &OnboardReportArtifact) -> String {
    let mut lines = vec![
        "# LoopForge Onboard Report".to_string(),
        "".to_string(),
        format!("- Workspace: `{}`", report.workspace),
        format!("- Starter: `{}`", report.starter),
        format!("- Config valid: {}", report.config_valid),
        format!(
            "- Doctor summary: ok={} warn={} error={}",
            report.doctor_summary.ok, report.doctor_summary.warn, report.doctor_summary.error
        ),
        format!("- First task status: `{}`", report.task.status),
        format!(
            "- Recommended next command: `{}`",
            report.recommended_next_command
        ),
        "".to_string(),
        "## Effective Prompt".to_string(),
        "".to_string(),
        "```text".to_string(),
        report.effective_prompt.clone(),
        "```".to_string(),
        "".to_string(),
    ];

    if !report.doctor_next_actions.is_empty() {
        lines.push("## Suggested Next Steps".to_string());
        lines.push("".to_string());
        for action in &report.doctor_next_actions {
            lines.push(format!("- {}", action));
        }
        lines.push("".to_string());
    }

    lines.push("## Starter Suggestions".to_string());
    lines.push("".to_string());
    for suggestion in &report.starter_suggestions {
        lines.push(format!(
            "- `{}` — {}",
            suggestion.command, suggestion.description
        ));
    }
    lines.push("".to_string());

    if let Some(session_id) = &report.task.session_id {
        lines.push("## Session".to_string());
        lines.push("".to_string());
        lines.push(format!("- Session ID: `{}`", session_id));
        lines.push("".to_string());
    }

    if let Some(error) = &report.task.error {
        lines.push("## Error".to_string());
        lines.push("".to_string());
        lines.push("```text".to_string());
        lines.push(error.clone());
        lines.push("```".to_string());
        lines.push("".to_string());
    }

    lines.join("\n")
}

fn write_onboard_report(
    workspace: &Path,
    report: &OnboardReportArtifact,
) -> anyhow::Result<(PathBuf, PathBuf)> {
    let (json_path, md_path) = onboard_report_paths(workspace);
    if let Some(parent) = json_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create onboard report dir: {}", parent.display()))?;
    }
    std::fs::write(&json_path, serde_json::to_string_pretty(report)? + "\n")
        .with_context(|| format!("write {}", json_path.display()))?;
    std::fs::write(&md_path, render_onboard_report_markdown(report) + "\n")
        .with_context(|| format!("write {}", md_path.display()))?;
    Ok((json_path, md_path))
}

fn print_onboard_report_summary(report: &OnboardReportArtifact, json_path: &Path, md_path: &Path) {
    println!("onboard summary:");
    println!(
        "- doctor: ok={} warn={} error={}",
        report.doctor_summary.ok, report.doctor_summary.warn, report.doctor_summary.error
    );
    println!("- task status: {}", report.task.status);
    println!("- report json: {}", json_path.display());
    println!("- report md: {}", md_path.display());
    println!("- next command: {}", report.recommended_next_command);
}

async fn fetch_openai_compat_models(
    base_url: &str,
    timeout_ms: u64,
) -> anyhow::Result<Vec<String>> {
    let endpoint = format!("{}/models", base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(timeout_ms.max(500)))
        .build()
        .context("build model probe http client")?;
    let res = client.get(&endpoint).send().await?;
    if !res.status().is_success() {
        anyhow::bail!("GET {endpoint} -> {}", res.status());
    }
    let v: serde_json::Value = res.json().await?;
    let mut out = Vec::new();
    if let Some(arr) = v.get("data").and_then(|x| x.as_array()) {
        for item in arr {
            if let Some(id) = item.get("id").and_then(|x| x.as_str()) {
                let id = id.trim();
                if !id.is_empty() {
                    out.push(id.to_string());
                    continue;
                }
            }
            if let Some(name) = item.get("name").and_then(|x| x.as_str()) {
                let name = name.trim();
                if !name.is_empty() {
                    out.push(name.to_string());
                }
            }
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn parse_release_tag_version(tag: &str) -> Option<String> {
    let tag = tag.trim();
    let version = tag.strip_prefix('v')?;
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    if parts
        .iter()
        .all(|p| !p.is_empty() && p.chars().all(|ch| ch.is_ascii_digit()))
    {
        Some(version.to_string())
    } else {
        None
    }
}

fn extract_workspace_version_from_toml(cargo_toml: &str) -> Option<String> {
    let value: toml::Value = toml::from_str(cargo_toml).ok()?;
    value
        .get("workspace")?
        .get("package")?
        .get("version")?
        .as_str()
        .map(|s| s.to_string())
}

fn changelog_has_release_section(changelog_text: &str, version: &str) -> bool {
    let target = format!("## [{version}]");
    changelog_text
        .lines()
        .any(|line| line.trim_start().starts_with(&target))
}

fn evaluate_release_metadata(
    cargo_toml: &str,
    changelog_text: &str,
    tag: &str,
) -> ReleaseCheckReport {
    let mut checks = Vec::new();

    let tag_version = parse_release_tag_version(tag);
    checks.push(ReleaseCheckItem {
        id: "tag.format".to_string(),
        ok: tag_version.is_some(),
        message: if tag_version.is_some() {
            format!("tag `{tag}` matches vX.Y.Z")
        } else {
            format!("tag `{tag}` is invalid; expected vX.Y.Z")
        },
    });

    let cargo_version = extract_workspace_version_from_toml(cargo_toml);
    checks.push(ReleaseCheckItem {
        id: "cargo.workspace_version".to_string(),
        ok: cargo_version.is_some(),
        message: match cargo_version.as_deref() {
            Some(v) => format!("workspace version `{v}`"),
            None => "failed to parse [workspace.package].version".to_string(),
        },
    });

    let versions_match = match (tag_version.as_deref(), cargo_version.as_deref()) {
        (Some(tag_v), Some(cargo_v)) => tag_v == cargo_v,
        _ => false,
    };
    checks.push(ReleaseCheckItem {
        id: "cargo.matches_tag".to_string(),
        ok: versions_match,
        message: match (tag_version.as_deref(), cargo_version.as_deref()) {
            (Some(tag_v), Some(cargo_v)) => {
                if tag_v == cargo_v {
                    format!("tag version `{tag_v}` matches Cargo.toml")
                } else {
                    format!("tag version `{tag_v}` does not match Cargo.toml `{cargo_v}`")
                }
            }
            _ => "cannot compare tag and Cargo.toml versions".to_string(),
        },
    });

    let changelog_ok = tag_version
        .as_deref()
        .map(|v| changelog_has_release_section(changelog_text, v))
        .unwrap_or(false);
    checks.push(ReleaseCheckItem {
        id: "changelog.section".to_string(),
        ok: changelog_ok,
        message: match tag_version.as_deref() {
            Some(v) if changelog_ok => format!("found changelog section [{v}]"),
            Some(v) => format!("missing changelog section [{v}]"),
            None => "cannot verify changelog without valid tag".to_string(),
        },
    });

    let ok = checks.iter().all(|c| c.ok);
    ReleaseCheckReport {
        ok,
        tag: tag.to_string(),
        checks,
    }
}

fn run_release_check(
    repo_root: &Path,
    tag: Option<&str>,
    run_tests: bool,
) -> anyhow::Result<ReleaseCheckReport> {
    let cargo_path = repo_root.join("Cargo.toml");
    let changelog_path = repo_root.join("CHANGELOG.md");

    let cargo_toml = std::fs::read_to_string(&cargo_path)
        .with_context(|| format!("read {}", cargo_path.display()))?;
    let changelog_text = std::fs::read_to_string(&changelog_path)
        .with_context(|| format!("read {}", changelog_path.display()))?;

    let default_tag = extract_workspace_version_from_toml(&cargo_toml)
        .map(|v| format!("v{v}"))
        .unwrap_or_else(|| "v0.0.0".to_string());
    let resolved_tag = tag.map(|s| s.to_string()).unwrap_or(default_tag);

    let mut report = evaluate_release_metadata(&cargo_toml, &changelog_text, &resolved_tag);

    for (id, rel_path) in [
        ("workflow.release", ".github/workflows/release.yml"),
        (
            "workflow.release_dry_run",
            ".github/workflows/release-dry-run.yml",
        ),
        ("script.package_release", "scripts/package_release.py"),
    ] {
        let full = repo_root.join(rel_path);
        let exists = full.exists();
        report.checks.push(ReleaseCheckItem {
            id: id.to_string(),
            ok: exists,
            message: if exists {
                format!("{rel_path} exists")
            } else {
                format!("{rel_path} is missing")
            },
        });
    }

    if run_tests {
        let status = ProcessCommand::new("cargo")
            .arg("test")
            .arg("--workspace")
            .arg("--locked")
            .current_dir(repo_root)
            .status()
            .context("run cargo test --workspace --locked")?;
        report.checks.push(ReleaseCheckItem {
            id: "preflight.tests".to_string(),
            ok: status.success(),
            message: format!("cargo test exit status: {status}"),
        });
    } else {
        report.checks.push(ReleaseCheckItem {
            id: "preflight.tests".to_string(),
            ok: true,
            message: "skipped (pass --run-tests to enable)".to_string(),
        });
    }

    report.ok = report.checks.iter().all(|c| c.ok);
    Ok(report)
}

fn format_release_check_report(report: &ReleaseCheckReport) -> String {
    let mut out = String::new();
    out.push_str(&format!("Release check for {}\n\n", report.tag));
    for check in &report.checks {
        let prefix = if check.ok { "OK  " } else { "ERR " };
        out.push_str(&format!("{prefix} {}: {}\n", check.id, check.message));
    }
    out.push_str(&format!(
        "\nSummary: {}\n",
        if report.ok { "PASS" } else { "FAIL" }
    ));
    out
}

fn load_acp_events(
    memory: &MemoryStore,
    session: Option<&str>,
    limit: usize,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let raw = memory
        .kv_get("rexos.acp.events")
        .context("kv_get rexos.acp.events")?
        .unwrap_or_else(|| "[]".to_string());
    let mut events: Vec<serde_json::Value> = serde_json::from_str(&raw).unwrap_or_default();

    if let Some(session) = session {
        let session = session.trim();
        if !session.is_empty() {
            events.retain(|ev| ev.get("session_id").and_then(|v| v.as_str()) == Some(session));
        }
    }

    let wanted = limit.max(1);
    if events.len() > wanted {
        events = events.split_off(events.len() - wanted);
    }
    Ok(events)
}

fn load_acp_checkpoints(
    memory: &MemoryStore,
    session: &str,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let session = session.trim();
    if session.is_empty() {
        anyhow::bail!("session is empty");
    }
    let key = format!("rexos.acp.checkpoints.{session}");
    let raw = memory
        .kv_get(&key)
        .with_context(|| format!("kv_get {key}"))?
        .unwrap_or_else(|| "[]".to_string());
    let checkpoints: Vec<serde_json::Value> = serde_json::from_str(&raw).unwrap_or_default();
    Ok(checkpoints)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn cli_primary_name_is_loopforge() {
        use clap::CommandFactory;
        assert_eq!(Cli::command().get_name(), "loopforge");
    }

    #[test]
    fn cli_about_uses_loopforge_only_branding() {
        use clap::CommandFactory;
        let about = Cli::command()
            .get_about()
            .map(|s| s.to_string())
            .unwrap_or_default();
        assert!(
            about.contains("LoopForge"),
            "expected LoopForge about text, got: {about}"
        );
        assert!(
            !about.contains("RexOS"),
            "expected no RexOS mention, got: {about}"
        );
    }

    #[test]
    fn cli_parses_config_validate_with_loopforge_binary_name() {
        let parsed = Cli::try_parse_from(["loopforge", "config", "validate"]);
        assert!(
            parsed.is_ok(),
            "expected `loopforge config validate` to parse, got: {parsed:?}"
        );
    }

    #[test]
    fn cli_parses_config_validate_subcommand() {
        let parsed = Cli::try_parse_from(["loopforge", "config", "validate"]);
        assert!(
            parsed.is_ok(),
            "expected `loopforge config validate` to parse, got: {parsed:?}"
        );
    }

    #[test]
    fn cli_parses_release_check_subcommand() {
        let parsed = Cli::try_parse_from(["loopforge", "release", "check", "--tag", "v0.1.0"]);
        assert!(
            parsed.is_ok(),
            "expected `loopforge release check` to parse, got: {parsed:?}"
        );
    }

    #[test]
    fn cli_parses_acp_events_subcommand() {
        let parsed = Cli::try_parse_from([
            "loopforge",
            "acp",
            "events",
            "--session",
            "s-1",
            "--limit",
            "20",
        ]);
        assert!(
            parsed.is_ok(),
            "expected `loopforge acp events` to parse, got: {parsed:?}"
        );
    }

    #[test]
    fn cli_parses_acp_checkpoints_subcommand() {
        let parsed = Cli::try_parse_from(["loopforge", "acp", "checkpoints", "--session", "s-1"]);
        assert!(
            parsed.is_ok(),
            "expected `loopforge acp checkpoints` to parse, got: {parsed:?}"
        );
    }

    #[test]
    fn cli_parses_agent_run_allowed_tools() {
        let parsed = Cli::try_parse_from([
            "loopforge",
            "agent",
            "run",
            "--workspace",
            ".",
            "--prompt",
            "x",
            "--allowed-tools",
            "fs_read,web_fetch",
        ]);
        assert!(
            parsed.is_ok(),
            "expected agent run with --allowed-tools to parse, got: {parsed:?}"
        );
    }

    #[test]
    fn cli_parses_skills_list_subcommand() {
        let parsed = Cli::try_parse_from(["loopforge", "skills", "list", "--workspace", "."]);
        assert!(
            parsed.is_ok(),
            "expected `loopforge skills list` to parse, got: {parsed:?}"
        );
    }

    #[test]
    fn cli_parses_skills_run_subcommand() {
        let parsed = Cli::try_parse_from([
            "loopforge",
            "skills",
            "run",
            "hello-skill",
            "--workspace",
            ".",
            "--input",
            "do x",
        ]);
        assert!(
            parsed.is_ok(),
            "expected `loopforge skills run` to parse, got: {parsed:?}"
        );
    }

    #[test]
    fn cli_parses_onboard_subcommand() {
        let parsed = Cli::try_parse_from([
            "loopforge",
            "onboard",
            "--workspace",
            "loopforge-onboard-demo",
        ]);
        assert!(
            parsed.is_ok(),
            "expected `loopforge onboard` to parse, got: {parsed:?}"
        );
    }

    #[test]
    fn cli_parses_onboard_starter_profile() {
        let parsed = Cli::try_parse_from([
            "loopforge",
            "onboard",
            "--workspace",
            "loopforge-onboard-demo",
            "--starter",
            "workspace-brief",
        ]);
        assert!(
            parsed.is_ok(),
            "expected `loopforge onboard --starter workspace-brief` to parse, got: {parsed:?}"
        );
    }

    #[test]
    fn release_metadata_check_passes_when_versions_match() {
        let cargo = r#"
[workspace]
members = []

[workspace.package]
version = "0.1.0"
edition = "2021"
"#;
        let changelog = "# Changelog\n\n## [0.1.0] - 2026-03-04\n";
        let report = evaluate_release_metadata(cargo, changelog, "v0.1.0");
        assert!(report.ok, "expected release metadata ok, got: {report:?}");
    }

    #[test]
    fn release_metadata_check_fails_when_changelog_missing_section() {
        let cargo = r#"
[workspace]
members = []

[workspace.package]
version = "0.1.0"
edition = "2021"
"#;
        let changelog = "# Changelog\n\n## [Unreleased]\n";
        let report = evaluate_release_metadata(cargo, changelog, "v0.1.0");
        assert!(
            !report.ok,
            "expected release metadata fail, got: {report:?}"
        );
        assert!(
            report
                .checks
                .iter()
                .any(|c| c.id == "changelog.section" && !c.ok),
            "expected changelog.section failure, got: {report:?}"
        );
    }

    #[test]
    fn validate_config_reports_success_for_default_config() {
        let tmp = tempdir().unwrap();
        let paths = RexosPaths {
            base_dir: tmp.path().join(".loopforge"),
        };
        paths.ensure_dirs().unwrap();
        RexosConfig::ensure_default(&paths).unwrap();

        let report = validate_config(&paths);
        assert!(report.valid, "expected config valid, got {report:?}");
        assert!(
            report.errors.is_empty(),
            "expected no errors, got {report:?}"
        );
    }

    #[test]
    fn validate_config_reports_parse_error_for_invalid_toml() {
        let tmp = tempdir().unwrap();
        let paths = RexosPaths {
            base_dir: tmp.path().join(".loopforge"),
        };
        paths.ensure_dirs().unwrap();
        std::fs::write(paths.config_path(), "[providers\nbroken = true").unwrap();

        let report = validate_config(&paths);
        assert!(!report.valid, "expected config invalid, got {report:?}");
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.contains("parse config TOML")),
            "expected parse error, got {report:?}"
        );
    }

    #[test]
    fn select_onboard_model_prefers_configured_when_available() {
        let selected = select_onboard_model(
            "llama3.2",
            &["qwen3:4b".to_string(), "llama3.2".to_string()],
        );
        assert_eq!(selected.as_deref(), Some("llama3.2"));
    }

    #[test]
    fn select_onboard_model_falls_back_to_first_non_embedding() {
        let selected = select_onboard_model(
            "llama3.2",
            &[
                "nomic-embed-text:latest".to_string(),
                "qwen3:4b".to_string(),
            ],
        );
        assert_eq!(selected.as_deref(), Some("qwen3:4b"));
    }

    #[test]
    fn select_onboard_model_uses_first_when_only_embedding_exists() {
        let selected = select_onboard_model("llama3.2", &["nomic-embed-text:latest".to_string()]);
        assert_eq!(selected.as_deref(), Some("nomic-embed-text:latest"));
    }

    #[test]
    fn resolve_onboard_prompt_uses_starter_when_prompt_missing() {
        let prompt = resolve_onboard_prompt(None, OnboardStarter::WorkspaceBrief);
        assert!(
            prompt.contains("workspace-brief") || prompt.contains("notes/workspace-brief.md"),
            "expected workspace brief prompt, got: {prompt}"
        );
    }

    #[test]
    fn resolve_onboard_prompt_prefers_explicit_prompt() {
        let prompt = resolve_onboard_prompt(Some("Create notes/custom.md"), OnboardStarter::Hello);
        assert_eq!(prompt, "Create notes/custom.md");
    }

    #[test]
    fn starter_expected_artifact_tracks_default_starters() {
        assert_eq!(
            starter_expected_artifact(None, OnboardStarter::WorkspaceBrief),
            Some("notes/workspace-brief.md")
        );
        assert_eq!(
            starter_expected_artifact(None, OnboardStarter::RepoOnboarding),
            Some("notes/repo-onboarding.md")
        );
        assert_eq!(
            starter_expected_artifact(Some("Create notes/custom.md"), OnboardStarter::Hello),
            None
        );
    }

    #[test]
    fn verify_onboard_artifact_requires_default_starter_output() {
        let tmp = tempdir().unwrap();
        let workspace = tmp.path().join("demo-work");
        std::fs::create_dir_all(&workspace).unwrap();

        let err = verify_onboard_artifact(&workspace, None, OnboardStarter::WorkspaceBrief)
            .expect_err("missing starter artifact should fail");
        assert!(err.to_string().contains("notes/workspace-brief.md"));

        std::fs::create_dir_all(workspace.join("notes")).unwrap();
        std::fs::write(workspace.join("notes/workspace-brief.md"), "brief").unwrap();
        assert_eq!(
            verify_onboard_artifact(&workspace, None, OnboardStarter::WorkspaceBrief).unwrap(),
            Some("notes/workspace-brief.md".to_string())
        );
        assert_eq!(
            verify_onboard_artifact(
                &workspace,
                Some("Create notes/custom.md"),
                OnboardStarter::WorkspaceBrief,
            )
            .unwrap(),
            None
        );
    }

    #[test]
    fn write_onboard_report_writes_json_and_markdown() {
        let tmp = tempdir().unwrap();
        let workspace = tmp.path().join("demo-work");
        std::fs::create_dir_all(&workspace).unwrap();
        let report = OnboardReportArtifact {
            generated_at_ms: 1,
            workspace: workspace.display().to_string(),
            config_path: "~/.loopforge/config.toml".to_string(),
            config_valid: true,
            starter: OnboardStarter::Hello.as_str().to_string(),
            effective_prompt: OnboardStarter::Hello.default_prompt().to_string(),
            doctor_summary: doctor::DoctorSummary {
                ok: 2,
                warn: 1,
                error: 0,
            },
            doctor_next_actions: vec![
                "Run `loopforge doctor` again after fixing config.".to_string()
            ],
            task: OnboardTaskReport {
                status: "skipped".to_string(),
                session_id: None,
                failure_category: None,
                error: None,
            },
            recommended_next_command: "loopforge onboard --workspace demo-work".to_string(),
            starter_suggestions: build_onboard_starter_suggestions(&workspace),
        };

        let (json_path, md_path) = write_onboard_report(&workspace, &report).unwrap();
        assert!(
            json_path.exists(),
            "expected json report at {}",
            json_path.display()
        );
        assert!(
            md_path.exists(),
            "expected markdown report at {}",
            md_path.display()
        );

        let md = std::fs::read_to_string(md_path).unwrap();
        assert!(md.contains("LoopForge Onboard Report"));
        assert!(md.contains("Suggested Next Steps"));
    }

    #[test]
    fn onboard_blocks_config_and_router_errors() {
        let config_error = doctor::DoctorCheck {
            id: "config.parse".to_string(),
            status: doctor::CheckStatus::Error,
            message: "bad toml".to_string(),
        };
        let router_error = doctor::DoctorCheck {
            id: "router.coding.provider".to_string(),
            status: doctor::CheckStatus::Error,
            message: "unknown provider".to_string(),
        };

        assert!(is_onboard_blocking_doctor_error(&config_error));
        assert!(is_onboard_blocking_doctor_error(&router_error));
    }

    #[test]
    fn onboard_does_not_block_non_critical_errors() {
        let git_error = doctor::DoctorCheck {
            id: "tools.git".to_string(),
            status: doctor::CheckStatus::Error,
            message: "git not found".to_string(),
        };
        let browser_warn = doctor::DoctorCheck {
            id: "browser.chromium".to_string(),
            status: doctor::CheckStatus::Warn,
            message: "missing".to_string(),
        };

        assert!(!is_onboard_blocking_doctor_error(&git_error));
        assert!(!is_onboard_blocking_doctor_error(&browser_warn));
    }

    #[test]
    fn classify_onboard_failure_groups_common_causes() {
        assert_eq!(
            classify_onboard_failure("model llama3.2 not found"),
            "model_unavailable"
        );
        assert_eq!(
            classify_onboard_failure("request timed out while calling http provider"),
            "provider_unreachable"
        );
        assert_eq!(
            classify_onboard_failure("tool call failed with invalid arguments"),
            "tool_runtime_error"
        );
    }

    #[test]
    fn record_onboard_attempt_updates_metrics_and_events() {
        let tmp = tempdir().unwrap();
        let paths = RexosPaths {
            base_dir: tmp.path().join(".loopforge"),
        };
        std::fs::create_dir_all(&paths.base_dir).unwrap();
        let workspace = tmp.path().join("demo-work");
        std::fs::create_dir_all(&workspace).unwrap();

        let m1 = record_onboard_attempt(&paths, &workspace, "s1", true, None, None).unwrap();
        assert_eq!(m1.attempted_first_task, 1);
        assert_eq!(m1.first_task_success, 1);
        assert_eq!(m1.first_task_failed, 0);

        let m2 = record_onboard_attempt(
            &paths,
            &workspace,
            "s2",
            false,
            Some("provider_unreachable"),
            Some("timeout"),
        )
        .unwrap();
        assert_eq!(m2.attempted_first_task, 2);
        assert_eq!(m2.first_task_success, 1);
        assert_eq!(m2.first_task_failed, 1);
        assert_eq!(m2.failure_by_category.get("provider_unreachable"), Some(&1));

        let events_raw = std::fs::read_to_string(onboard_events_path(&paths)).unwrap();
        assert_eq!(events_raw.lines().count(), 2);
    }
}
