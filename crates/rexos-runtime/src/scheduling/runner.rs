use anyhow::Context;

use crate::records::{ChannelSendToolArgs, CronJobRecord, EventPublishToolArgs};
use crate::AgentRuntime;

#[derive(Debug, Clone)]
pub struct CronRunnerConfig {
    pub tick_interval_secs: u64,
    pub max_due_per_tick: usize,
    pub max_catchup_slots_per_job: u32,
    pub max_consecutive_errors: u32,
    pub min_retry_delay_secs: u64,
    pub job_timeout_ms: u64,
}

impl Default for CronRunnerConfig {
    fn default() -> Self {
        Self {
            tick_interval_secs: 2,
            max_due_per_tick: 20,
            max_catchup_slots_per_job: 25,
            max_consecutive_errors: 5,
            min_retry_delay_secs: 30,
            job_timeout_ms: 10_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronRunnerTickSummary {
    pub tick_at: i64,
    pub due: usize,
    pub ran: usize,
    pub ok: usize,
    pub failed: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CronSchedule {
    Every { every_secs: u64 },
    At { at_epoch_seconds: i64 },
}

fn parse_schedule(value: &serde_json::Value) -> anyhow::Result<CronSchedule> {
    match value {
        serde_json::Value::Object(map) => {
            let kind = map
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            match kind.as_str() {
                "every" => {
                    let every_secs = map
                        .get("every_secs")
                        .or_else(|| map.get("interval_secs"))
                        .and_then(|v| v.as_u64())
                        .context("schedule.every missing every_secs")?;
                    if every_secs == 0 {
                        anyhow::bail!("schedule.every every_secs must be > 0");
                    }
                    Ok(CronSchedule::Every { every_secs })
                }
                "at" => {
                    let at_epoch_seconds = map
                        .get("at_epoch_seconds")
                        .or_else(|| map.get("at"))
                        .and_then(|v| v.as_i64())
                        .context("schedule.at missing at_epoch_seconds")?;
                    Ok(CronSchedule::At { at_epoch_seconds })
                }
                other => anyhow::bail!("unsupported schedule kind: {other}"),
            }
        }
        _ => anyhow::bail!("schedule must be an object"),
    }
}

fn compute_initial_next_run_at(schedule: &CronSchedule, now: i64) -> i64 {
    match schedule {
        CronSchedule::Every { every_secs } => now.saturating_add(*every_secs as i64),
        CronSchedule::At { at_epoch_seconds } => *at_epoch_seconds,
    }
}

fn compute_next_run_after(schedule: &CronSchedule, scheduled_at: i64) -> Option<i64> {
    match schedule {
        CronSchedule::Every { every_secs } => Some(scheduled_at.saturating_add(*every_secs as i64)),
        CronSchedule::At { .. } => None,
    }
}

fn action_kind(value: &serde_json::Value) -> Option<&str> {
    match value {
        serde_json::Value::String(s) => Some(s.as_str()),
        serde_json::Value::Object(map) => map.get("kind").and_then(|v| v.as_str()),
        _ => None,
    }
}

fn default_system_event_type(job: &CronJobRecord) -> String {
    let cleaned = job.name.trim().replace(|c: char| c.is_whitespace(), "_");
    if cleaned.is_empty() {
        format!("cron.{}", job.job_id)
    } else {
        format!("cron.{cleaned}")
    }
}

fn truncate_status(s: String) -> String {
    const MAX: usize = 256;
    if s.chars().count() <= MAX {
        return s;
    }
    let mut out = String::new();
    for (idx, ch) in s.chars().enumerate() {
        if idx >= MAX {
            break;
        }
        out.push(ch);
    }
    out.push_str("…");
    out
}

impl AgentRuntime {
    pub async fn cron_runner_loop(&self, config: CronRunnerConfig) -> anyhow::Result<()> {
        self.cron_runner_recover_interrupted().await?;

        // Run one tick immediately, then follow interval ticks.
        let _ = self.cron_runner_tick(&config).await?;

        let mut interval = tokio::time::interval(std::time::Duration::from_secs(
            config.tick_interval_secs.max(1),
        ));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        interval.tick().await; // skip first immediate tick
        loop {
            interval.tick().await;
            let _ = self.cron_runner_tick(&config).await?;
        }
    }

    pub async fn cron_runner_tick(
        &self,
        config: &CronRunnerConfig,
    ) -> anyhow::Result<CronRunnerTickSummary> {
        let now = Self::now_epoch_seconds();
        self.cron_runner_tick_at(now, config).await
    }

    pub async fn cron_runner_tick_at(
        &self,
        now: i64,
        config: &CronRunnerConfig,
    ) -> anyhow::Result<CronRunnerTickSummary> {
        // Initialize next_run_at for newly created enabled jobs.
        let jobs = self.cron_jobs_get()?;
        for job in &jobs {
            if !job.enabled || job.next_run_at.is_some() {
                continue;
            }
            if let Ok(schedule) = parse_schedule(&job.schedule) {
                let next = compute_initial_next_run_at(&schedule, now);
                let job_id = job.job_id.clone();
                let _ = self.cron_jobs_update(|jobs| {
                    if let Some(existing) = jobs.iter_mut().find(|j| j.job_id == job_id) {
                        if existing.next_run_at.is_none() && existing.enabled {
                            existing.next_run_at = Some(next);
                        }
                    }
                    Ok(())
                })?;
            }
        }

        let jobs = self.cron_jobs_get()?;
        let mut due: Vec<(String, i64)> = Vec::new();
        for job in &jobs {
            if !job.enabled {
                continue;
            }
            let schedule = match parse_schedule(&job.schedule) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let next = match job.next_run_at {
                Some(t) => t,
                None => compute_initial_next_run_at(&schedule, now),
            };
            if next <= now {
                due.push((job.job_id.clone(), next));
            }
        }

        due.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        if due.len() > config.max_due_per_tick {
            due.truncate(config.max_due_per_tick);
        }

        let mut summary = CronRunnerTickSummary {
            tick_at: now,
            due: due.len(),
            ran: 0,
            ok: 0,
            failed: 0,
            skipped: 0,
        };

        for (job_id, scheduled_at) in due {
            match self
                .cron_runner_execute_slot(&job_id, scheduled_at, now, config)
                .await
            {
                Ok(outcome) => match outcome.as_str() {
                    "ok" => {
                        summary.ran += 1;
                        summary.ok += 1;
                    }
                    "skipped" => {
                        summary.skipped += 1;
                    }
                    _ => {
                        summary.ran += 1;
                        summary.failed += 1;
                    }
                },
                Err(_) => {
                    summary.ran += 1;
                    summary.failed += 1;
                }
            }
        }

        Ok(summary)
    }

    async fn cron_runner_recover_interrupted(&self) -> anyhow::Result<()> {
        let now = Self::now_epoch_seconds();
        self.cron_jobs_update(|jobs| {
            for job in jobs.iter_mut() {
                let Some(scheduled_at) = job.running_scheduled_at else {
                    continue;
                };

                let one_shot = job.one_shot
                    || matches!(parse_schedule(&job.schedule), Ok(CronSchedule::At { .. }));
                job.running_started_at = None;
                job.running_scheduled_at = None;

                if one_shot {
                    job.enabled = false;
                    job.last_status = Some("error: interrupted one-shot job dropped".to_string());
                    job.next_run_at = None;
                } else {
                    job.next_run_at = Some(scheduled_at);
                    job.last_status = Some("warn: recovered interrupted run".to_string());
                }

                job.consecutive_errors = job.consecutive_errors.saturating_add(1);
                job.last_run_at = Some(now);
            }
            Ok(())
        })?;
        Ok(())
    }

    async fn cron_runner_execute_slot(
        &self,
        job_id: &str,
        scheduled_at: i64,
        now: i64,
        config: &CronRunnerConfig,
    ) -> anyhow::Result<String> {
        // Mark running and pre-advance next_run_at so the job won't be re-selected while executing.
        let mut job_snapshot: Option<CronJobRecord> = None;
        self.cron_jobs_update(|jobs| {
            let Some(job) = jobs.iter_mut().find(|job| job.job_id == job_id) else {
                return Ok(());
            };
            if !job.enabled {
                return Ok(());
            }
            if job.running_started_at.is_some() {
                return Ok(());
            }

            let schedule = match parse_schedule(&job.schedule) {
                Ok(s) => s,
                Err(e) => {
                    job.last_status = Some(truncate_status(format!("error: {e}")));
                    return Ok(());
                }
            };

            let effective_next = job
                .next_run_at
                .unwrap_or_else(|| compute_initial_next_run_at(&schedule, now));
            if effective_next > now {
                job.next_run_at = Some(effective_next);
                return Ok(());
            }

            // If backlog is huge, trim to the last N slots to avoid unbounded catch-up storms.
            if let CronSchedule::Every { every_secs } = schedule {
                let max_slots = config.max_catchup_slots_per_job.max(1) as i64;
                let backlog_slots = (now.saturating_sub(effective_next) / every_secs as i64) + 1;
                if backlog_slots > max_slots {
                    let skipped = backlog_slots - max_slots;
                    let new_next = now.saturating_sub((max_slots - 1) * every_secs as i64);
                    job.last_status =
                        Some(format!("warn: catch-up trimmed; skipped {skipped} slot(s)"));
                    job.next_run_at = Some(new_next);
                }
            }

            let schedule = parse_schedule(&job.schedule)?;
            let effective_next = job
                .next_run_at
                .unwrap_or_else(|| compute_initial_next_run_at(&schedule, now));
            if effective_next != scheduled_at {
                job.next_run_at = Some(effective_next);
                job.running_started_at = None;
                job.running_scheduled_at = None;
                return Ok(());
            }

            job.running_started_at = Some(now);
            job.running_scheduled_at = Some(effective_next);
            job_snapshot = Some(job.clone());
            job.next_run_at = compute_next_run_after(&schedule, effective_next);
            Ok(())
        })?;

        let Some(job) = job_snapshot else {
            return Ok("skipped".to_string());
        };

        let timeout = std::time::Duration::from_millis(config.job_timeout_ms.max(1));
        let action_res = tokio::time::timeout(timeout, self.cron_runner_fire(&job, now))
            .await
            .map_err(|_| anyhow::anyhow!("job timeout after {}ms", timeout.as_millis()))?;

        match action_res {
            Ok(status) => {
                self.cron_jobs_update(|jobs| {
                    let Some(existing) = jobs.iter_mut().find(|j| j.job_id == job.job_id) else {
                        return Ok(());
                    };

                    existing.running_started_at = None;
                    existing.running_scheduled_at = None;
                    existing.last_run_at = job.running_scheduled_at;
                    existing.last_status = Some(truncate_status(status.clone()));
                    existing.consecutive_errors = 0;

                    let disable_after_success = match parse_schedule(&existing.schedule) {
                        Ok(CronSchedule::At { .. }) => true,
                        _ => false,
                    };
                    if existing.one_shot || disable_after_success {
                        existing.enabled = false;
                        existing.next_run_at = None;
                    }

                    Ok(())
                })?;
                Ok("ok".to_string())
            }
            Err(err) => {
                let err_s = truncate_status(format!("error: {err}"));
                self.cron_jobs_update(|jobs| {
                    let Some(existing) = jobs.iter_mut().find(|j| j.job_id == job.job_id) else {
                        return Ok(());
                    };

                    existing.running_started_at = None;
                    existing.running_scheduled_at = None;
                    existing.last_run_at = job.running_scheduled_at;
                    existing.last_status = Some(err_s.clone());
                    existing.consecutive_errors = existing.consecutive_errors.saturating_add(1);

                    if existing.consecutive_errors >= config.max_consecutive_errors.max(1) {
                        existing.enabled = false;
                        existing.next_run_at = None;
                    } else {
                        let schedule = match parse_schedule(&existing.schedule) {
                            Ok(s) => s,
                            Err(_) => return Ok(()),
                        };
                        let min_next = now.saturating_add(config.min_retry_delay_secs as i64);
                        let next =
                            compute_next_run_after(&schedule, existing.last_run_at.unwrap_or(now))
                                .unwrap_or(min_next);
                        existing.next_run_at = Some(next.max(min_next));
                    }

                    Ok(())
                })?;
                Ok("failed".to_string())
            }
        }
    }

    async fn cron_runner_fire(&self, job: &CronJobRecord, now: i64) -> anyhow::Result<String> {
        let kind = action_kind(&job.action).unwrap_or("").trim();
        match kind {
            "system_event" => {
                let event_type = match &job.action {
                    serde_json::Value::Object(map) => map
                        .get("event_type")
                        .and_then(|v| v.as_str())
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| default_system_event_type(job)),
                    _ => default_system_event_type(job),
                };

                let mut payload = serde_json::json!({
                    "job_id": job.job_id.clone(),
                    "job_name": job.name.clone(),
                    "scheduled_at": job.running_scheduled_at,
                    "fired_at": now,
                });

                if let serde_json::Value::Object(map) = &job.action {
                    if let Some(text) = map.get("text").and_then(|v| v.as_str()) {
                        payload["text"] = serde_json::Value::String(text.to_string());
                    }
                    if let Some(extra) = map.get("payload") {
                        payload["payload"] = extra.clone();
                    }
                }

                let _ = self.event_publish(EventPublishToolArgs {
                    event_type,
                    payload: Some(payload),
                })?;
                Ok("ok".to_string())
            }
            "channel_send" => {
                let mut merged: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
                if let Some(serde_json::Value::Object(map)) = job.delivery.as_ref() {
                    for (k, v) in map {
                        merged.insert(k.clone(), v.clone());
                    }
                }
                if let serde_json::Value::Object(map) = &job.action {
                    for key in ["channel", "recipient", "subject", "message"] {
                        if let Some(v) = map.get(key) {
                            merged.insert(key.to_string(), v.clone());
                        }
                    }
                }

                let channel = merged
                    .get("channel")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let recipient = merged
                    .get("recipient")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let subject = merged
                    .get("subject")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let message = merged
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let out = self.channel_send(
                    None,
                    ChannelSendToolArgs {
                        channel,
                        recipient,
                        subject,
                        message,
                    },
                )?;
                if out.trim_start().starts_with("error:") {
                    anyhow::bail!("{out}");
                }
                Ok(out)
            }
            other => anyhow::bail!("unsupported action kind: {other}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::*;
    use crate::AgentRuntime;
    use rexos_kernel::config::{LlmConfig, RexosConfig, RouterConfig};
    use rexos_kernel::paths::RexosPaths;
    use rexos_kernel::router::ModelRouter;
    use rexos_llm::registry::LlmRegistry;
    use rexos_memory::MemoryStore;

    fn make_agent() -> (tempfile::TempDir, AgentRuntime) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let paths = RexosPaths {
            base_dir: tmp.path().join(".loopforge"),
        };
        paths.ensure_dirs().expect("ensure dirs");
        let memory = MemoryStore::open_or_create(&paths).expect("open memory");

        let cfg = RexosConfig {
            llm: LlmConfig::default(),
            providers: BTreeMap::new(),
            router: RouterConfig::default(),
            security: Default::default(),
        };
        let llms = LlmRegistry::from_config(&cfg).expect("llm registry");
        let router = ModelRouter::new(cfg.router);
        let agent = AgentRuntime::new_with_security_config(memory, llms, router, cfg.security);
        (tmp, agent)
    }

    #[tokio::test]
    async fn cron_runner_initializes_next_run_for_new_every_job() {
        let (_tmp, agent) = make_agent();
        let job = CronJobRecord {
            job_id: "job1".to_string(),
            name: "demo".to_string(),
            schedule: json!({"kind":"every","every_secs":60}),
            action: json!({"kind":"system_event","text":"ping"}),
            delivery: None,
            one_shot: false,
            created_at: 0,
            enabled: true,
            last_run_at: None,
            next_run_at: None,
            last_status: None,
            consecutive_errors: 0,
            running_started_at: None,
            running_scheduled_at: None,
        };
        agent.cron_jobs_set(&[job]).unwrap();

        let cfg = CronRunnerConfig {
            tick_interval_secs: 1,
            ..Default::default()
        };
        let now = 1_000;
        let summary = agent.cron_runner_tick_at(now, &cfg).await.unwrap();
        assert_eq!(summary.due, 0, "{summary:?}");

        let jobs = agent.cron_jobs_get().unwrap();
        let j = jobs.iter().find(|j| j.job_id == "job1").unwrap();
        assert_eq!(j.next_run_at, Some(now + 60), "{j:?}");
    }

    #[tokio::test]
    async fn cron_runner_fires_system_event_for_due_job() {
        let (_tmp, agent) = make_agent();
        let job = CronJobRecord {
            job_id: "job1".to_string(),
            name: "demo".to_string(),
            schedule: json!({"kind":"every","every_secs":60}),
            action: json!({"kind":"system_event","text":"ping"}),
            delivery: None,
            one_shot: false,
            created_at: 0,
            enabled: true,
            last_run_at: None,
            next_run_at: Some(1_000),
            last_status: None,
            consecutive_errors: 0,
            running_started_at: None,
            running_scheduled_at: None,
        };
        agent.cron_jobs_set(&[job]).unwrap();

        let cfg = CronRunnerConfig {
            tick_interval_secs: 1,
            ..Default::default()
        };
        let now = 1_000;
        let summary = agent.cron_runner_tick_at(now, &cfg).await.unwrap();
        assert_eq!(summary.ok, 1, "{summary:?}");

        let raw = agent
            .memory
            .kv_get("rexos.events")
            .unwrap()
            .unwrap_or_default();
        let events: Vec<serde_json::Value> = serde_json::from_str(&raw).unwrap_or_default();
        assert_eq!(events.len(), 1, "{events:?}");
        assert_eq!(
            events[0].get("event_type").and_then(|v| v.as_str()),
            Some("cron.demo"),
            "{events:?}"
        );

        let jobs = agent.cron_jobs_get().unwrap();
        let j = jobs.iter().find(|j| j.job_id == "job1").unwrap();
        assert_eq!(j.last_run_at, Some(now), "{j:?}");
        assert_eq!(j.next_run_at, Some(now + 60), "{j:?}");
        assert_eq!(j.consecutive_errors, 0, "{j:?}");
    }

    #[tokio::test]
    async fn cron_runner_treats_channel_send_validation_errors_as_failures() {
        let (_tmp, agent) = make_agent();
        let job = CronJobRecord {
            job_id: "job1".to_string(),
            name: "demo".to_string(),
            schedule: json!({"kind":"every","every_secs":1}),
            action: json!({"kind":"channel_send"}),
            delivery: Some(json!({})),
            one_shot: false,
            created_at: 0,
            enabled: true,
            last_run_at: None,
            next_run_at: Some(1_000),
            last_status: None,
            consecutive_errors: 0,
            running_started_at: None,
            running_scheduled_at: None,
        };
        agent.cron_jobs_set(&[job]).unwrap();

        let cfg = CronRunnerConfig {
            tick_interval_secs: 1,
            min_retry_delay_secs: 30,
            ..Default::default()
        };
        let now = 1_000;
        let summary = agent.cron_runner_tick_at(now, &cfg).await.unwrap();
        assert_eq!(summary.failed, 1, "{summary:?}");

        let jobs = agent.cron_jobs_get().unwrap();
        let j = jobs.iter().find(|j| j.job_id == "job1").unwrap();
        assert_eq!(j.consecutive_errors, 1, "{j:?}");
        assert!(
            j.last_status
                .as_deref()
                .unwrap_or_default()
                .starts_with("error:"),
            "{j:?}"
        );
        assert_eq!(j.next_run_at, Some(now + 30), "{j:?}");
    }

    #[tokio::test]
    async fn cron_runner_recovers_interrupted_recurring_jobs() {
        let (_tmp, agent) = make_agent();
        let job = CronJobRecord {
            job_id: "job1".to_string(),
            name: "demo".to_string(),
            schedule: json!({"kind":"every","every_secs":60}),
            action: json!({"kind":"system_event","text":"ping"}),
            delivery: None,
            one_shot: false,
            created_at: 0,
            enabled: true,
            last_run_at: None,
            next_run_at: Some(10_000),
            last_status: None,
            consecutive_errors: 0,
            running_started_at: Some(900),
            running_scheduled_at: Some(1_234),
        };
        agent.cron_jobs_set(&[job]).unwrap();

        agent.cron_runner_recover_interrupted().await.unwrap();

        let jobs = agent.cron_jobs_get().unwrap();
        let j = jobs.iter().find(|j| j.job_id == "job1").unwrap();
        assert_eq!(j.running_started_at, None, "{j:?}");
        assert_eq!(j.running_scheduled_at, None, "{j:?}");
        assert_eq!(j.next_run_at, Some(1_234), "{j:?}");
        assert!(
            j.last_status
                .as_deref()
                .unwrap_or_default()
                .starts_with("warn:"),
            "{j:?}"
        );
        assert_eq!(j.consecutive_errors, 1, "{j:?}");
    }
}
