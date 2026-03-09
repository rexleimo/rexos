use crate::{cli::CronCommand, runtime_env};

pub(super) async fn run(command: CronCommand) -> anyhow::Result<()> {
    let (_paths, agent) = runtime_env::load_agent_runtime()?;

    match command {
        CronCommand::Tick {
            max_due_per_tick,
            max_catchup_slots_per_job,
            max_consecutive_errors,
            min_retry_delay_secs,
            job_timeout_ms,
        } => {
            let config = rexos::agent::CronRunnerConfig {
                tick_interval_secs: 1,
                max_due_per_tick,
                max_catchup_slots_per_job,
                max_consecutive_errors,
                min_retry_delay_secs,
                job_timeout_ms,
            };
            let summary = agent.cron_runner_tick(&config).await?;
            println!(
                "cron: tick_at={} due={} ran={} ok={} failed={} skipped={}",
                summary.tick_at,
                summary.due,
                summary.ran,
                summary.ok,
                summary.failed,
                summary.skipped
            );
            Ok(())
        }
        CronCommand::Worker {
            interval_secs,
            max_due_per_tick,
            max_catchup_slots_per_job,
            max_consecutive_errors,
            min_retry_delay_secs,
            job_timeout_ms,
        } => {
            let config = rexos::agent::CronRunnerConfig {
                tick_interval_secs: interval_secs,
                max_due_per_tick,
                max_catchup_slots_per_job,
                max_consecutive_errors,
                min_retry_delay_secs,
                job_timeout_ms,
            };
            agent.cron_runner_loop(config).await?;
            Ok(())
        }
    }
}
