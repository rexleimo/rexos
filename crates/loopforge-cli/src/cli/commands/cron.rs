#[derive(Debug, clap::Subcommand)]
pub(crate) enum CronCommand {
    /// Run one cron scheduler tick (process due jobs once)
    Tick {
        /// Max due jobs to execute in this tick
        #[arg(long, default_value_t = 20)]
        max_due_per_tick: usize,
        /// Max catch-up slots per job when recovering after downtime
        #[arg(long, default_value_t = 25)]
        max_catchup_slots_per_job: u32,
        /// Auto-disable a job after this many consecutive failures
        #[arg(long, default_value_t = 5)]
        max_consecutive_errors: u32,
        /// Minimum delay (seconds) before retrying a failing job
        #[arg(long, default_value_t = 30)]
        min_retry_delay_secs: u64,
        /// Per-job execution timeout (milliseconds)
        #[arg(long, default_value_t = 10_000)]
        job_timeout_ms: u64,
    },
    /// Run a long-lived cron worker loop
    Worker {
        /// Seconds between scheduler ticks
        #[arg(long, default_value_t = 2)]
        interval_secs: u64,
        /// Max due jobs to execute per tick
        #[arg(long, default_value_t = 20)]
        max_due_per_tick: usize,
        /// Max catch-up slots per job when recovering after downtime
        #[arg(long, default_value_t = 25)]
        max_catchup_slots_per_job: u32,
        /// Auto-disable a job after this many consecutive failures
        #[arg(long, default_value_t = 5)]
        max_consecutive_errors: u32,
        /// Minimum delay (seconds) before retrying a failing job
        #[arg(long, default_value_t = 30)]
        min_retry_delay_secs: u64,
        /// Per-job execution timeout (milliseconds)
        #[arg(long, default_value_t = 10_000)]
        job_timeout_ms: u64,
    },
}
