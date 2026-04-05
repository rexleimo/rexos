use anyhow::bail;

use crate::approval::{skill_approval_is_granted, skill_permissions_are_readonly};
use crate::records::{AcpEventRecord, SkillAuditRecord};
use crate::{AgentRuntime, SessionSkillPolicy};

impl AgentRuntime {
    pub fn record_skill_discovered(
        &self,
        session_id: &str,
        skill_name: &str,
        source: &str,
        version: &str,
    ) -> anyhow::Result<()> {
        self.append_acp_event(AcpEventRecord {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: Some(session_id.to_string()),
            event_type: "skill.discovered".to_string(),
            payload: serde_json::json!({
                "skill": skill_name,
                "source": source,
                "version": version,
            }),
            created_at: Self::now_epoch_seconds(),
        })
    }

    pub fn authorize_skill(
        &self,
        session_id: &str,
        skill_name: &str,
        requested_permissions: &[String],
    ) -> anyhow::Result<()> {
        let session_policy = self.load_session_policy_snapshot(session_id)?;

        if let Some(allowed_skills) = session_policy.allowed_skills.as_ref() {
            if !allowed_skills
                .iter()
                .any(|skill| skill.eq_ignore_ascii_case(skill_name.trim()))
            {
                let msg = format!("skill not allowed for this session: {skill_name}");
                let _ = self.append_acp_event(AcpEventRecord {
                    id: uuid::Uuid::new_v4().to_string(),
                    session_id: Some(session_id.to_string()),
                    event_type: "skill.blocked".to_string(),
                    payload: serde_json::json!({
                        "skill": skill_name,
                        "reason": "session_whitelist",
                        "message": msg,
                    }),
                    created_at: Self::now_epoch_seconds(),
                });
                bail!("{msg}");
            }
        }

        let policy: SessionSkillPolicy = session_policy.skill_policy;
        if !policy.allowlist.is_empty()
            && !policy
                .allowlist
                .iter()
                .any(|skill| skill.eq_ignore_ascii_case(skill_name.trim()))
        {
            let msg = format!("skill blocked by policy allowlist: {skill_name}");
            let _ = self.append_acp_event(AcpEventRecord {
                id: uuid::Uuid::new_v4().to_string(),
                session_id: Some(session_id.to_string()),
                event_type: "skill.blocked".to_string(),
                payload: serde_json::json!({
                    "skill": skill_name,
                    "reason": "policy_allowlist",
                    "message": msg,
                }),
                created_at: Self::now_epoch_seconds(),
            });
            bail!("{msg}");
        }

        if policy.require_approval
            && !(policy.auto_approve_readonly
                && skill_permissions_are_readonly(requested_permissions))
            && !skill_approval_is_granted(skill_name)
        {
            let msg = format!(
                "approval required for skill `{skill_name}` (set LOOPFORGE_SKILL_APPROVAL_ALLOW={skill_name} or all)"
            );
            let _ = self.append_acp_event(AcpEventRecord {
                id: uuid::Uuid::new_v4().to_string(),
                session_id: Some(session_id.to_string()),
                event_type: "skill.blocked".to_string(),
                payload: serde_json::json!({
                    "skill": skill_name,
                    "reason": "approval_required",
                    "message": msg,
                }),
                created_at: Self::now_epoch_seconds(),
            });
            bail!("{msg}");
        }

        self.append_acp_event(AcpEventRecord {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: Some(session_id.to_string()),
            event_type: "skill.loaded".to_string(),
            payload: serde_json::json!({
                "skill": skill_name,
                "permissions": requested_permissions,
            }),
            created_at: Self::now_epoch_seconds(),
        })?;
        Ok(())
    }

    pub fn record_skill_execution(
        &self,
        session_id: &str,
        skill_name: &str,
        requested_permissions: &[String],
        success: bool,
        error: Option<&str>,
    ) -> anyhow::Result<()> {
        let event_type = if success {
            "skill.executed"
        } else {
            "skill.failed"
        };
        self.append_acp_event(AcpEventRecord {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: Some(session_id.to_string()),
            event_type: event_type.to_string(),
            payload: serde_json::json!({
                "skill": skill_name,
                "permissions": requested_permissions,
                "error": error,
            }),
            created_at: Self::now_epoch_seconds(),
        })?;

        self.append_skill_audit(SkillAuditRecord {
            session_id: session_id.to_string(),
            skill_name: skill_name.to_string(),
            success,
            permissions: requested_permissions.to_vec(),
            error: error.map(|value| value.to_string()),
            created_at: Self::now_epoch_seconds(),
        })
    }
}
