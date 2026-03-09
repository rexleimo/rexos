mod acp;
mod agent;
mod channel;
mod config;
mod cron;
mod daemon;
mod harness;
mod release;
mod skills;

use clap::Parser;
use std::path::PathBuf;

use crate::onboard::OnboardStarter;

pub(crate) use acp::AcpCommand;
pub(crate) use agent::{AgentCommand, AgentKind};
pub(crate) use channel::ChannelCommand;
pub(crate) use config::ConfigCommand;
pub(crate) use cron::CronCommand;
pub(crate) use daemon::DaemonCommand;
pub(crate) use harness::HarnessCommand;
pub(crate) use release::ReleaseCommand;
pub(crate) use skills::SkillsCommand;

#[derive(Debug, Parser)]
#[command(name = "loopforge")]
#[command(
    about = "LoopForge: long-running agent operating system",
    long_about = None
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
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
    /// Cron scheduler helpers (stored jobs + optional runner)
    Cron {
        #[command(subcommand)]
        command: CronCommand,
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
