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
    Init { dir: PathBuf },
    /// Run a harness preflight session (bearings + smoke checks)
    Run { dir: PathBuf },
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
        Command::Harness { command } => match command {
            HarnessCommand::Init { dir } => {
                rexos::harness::init_workspace(&dir)?;
                println!("Harness initialized in {}", dir.display());
            }
            HarnessCommand::Run { dir } => {
                rexos::harness::preflight(&dir)?;
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
