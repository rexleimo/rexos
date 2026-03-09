mod acp;
mod agent;
mod channel;
mod config;
mod cron;
mod daemon;
mod doctor;
mod harness;
mod init;
mod release;
mod skills;

use crate::{
    cli::{Cli, Command},
    onboard,
};

pub(crate) async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Init => init::run(),
        Command::Onboard {
            workspace,
            prompt,
            starter,
            skip_agent,
            timeout_ms,
        } => onboard::run(workspace, prompt, starter, skip_agent, timeout_ms).await,
        Command::Doctor {
            json,
            strict,
            timeout_ms,
        } => doctor::run(json, strict, timeout_ms).await,
        Command::Agent { command } => agent::run(command).await,
        Command::Channel { command } => channel::run(command).await,
        Command::Cron { command } => cron::run(command).await,
        Command::Acp { command } => acp::run(command),
        Command::Config { command } => config::run(command),
        Command::Skills { command } => skills::run(command).await,
        Command::Harness { command } => harness::run(command).await,
        Command::Daemon { command } => daemon::run(command).await,
        Command::Release { command } => release::run(command),
    }
}
