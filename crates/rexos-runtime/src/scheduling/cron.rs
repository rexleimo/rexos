use anyhow::Context;

use crate::records::{CronCreateToolArgs, CronJobRecord};
use crate::AgentRuntime;

impl AgentRuntime {
    pub(crate) fn cron_create(&self, args: CronCreateToolArgs) -> anyhow::Result<String> {
        let job_id = args
            .job_id
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        self.cron_jobs_update(|jobs| {
            if let Some(existing) = jobs.iter().find(|job| job.job_id == job_id) {
                return Ok(serde_json::to_string(existing).unwrap_or_else(|_| "ok".to_string()));
            }

            let record = CronJobRecord {
                job_id: job_id.clone(),
                name: args.name,
                schedule: args.schedule,
                action: args.action,
                delivery: args.delivery,
                one_shot: args.one_shot.unwrap_or(false),
                created_at: Self::now_epoch_seconds(),
                enabled: args.enabled.unwrap_or(true),
                last_run_at: None,
                next_run_at: None,
                last_status: None,
                consecutive_errors: 0,
                running_started_at: None,
                running_scheduled_at: None,
            };

            jobs.push(record.clone());
            if jobs.len() > 200 {
                jobs.drain(0..(jobs.len() - 200));
            }

            Ok(serde_json::to_string(&record).unwrap_or_else(|_| "ok".to_string()))
        })
    }

    pub(crate) fn cron_list(&self) -> anyhow::Result<String> {
        let jobs = self.cron_jobs_get()?;
        Ok(serde_json::to_string(&jobs).context("serialize cron_list")?)
    }

    pub(crate) fn cron_cancel(&self, job_id: &str) -> anyhow::Result<String> {
        self.cron_jobs_update(|jobs| {
            let before = jobs.len();
            jobs.retain(|job| job.job_id != job_id);
            if jobs.len() == before {
                return Ok(format!("error: cron job not found: {job_id}"));
            }
            Ok("ok".to_string())
        })
    }
}
