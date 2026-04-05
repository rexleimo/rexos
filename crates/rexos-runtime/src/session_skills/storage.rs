use std::collections::HashSet;

use anyhow::Context;

use crate::{
    AgentRuntime, SessionSkillPolicy, SESSION_ALLOWED_SKILLS_KEY_PREFIX,
    SESSION_ALLOWED_TOOLS_KEY_PREFIX, SESSION_MCP_CONFIG_KEY_PREFIX,
    SESSION_SKILL_POLICY_KEY_PREFIX,
};

#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionPolicySnapshot {
    pub allowed_tools: Option<Vec<String>>,
    pub allowed_skills: Option<Vec<String>>,
    pub skill_policy: SessionSkillPolicy,
    pub mcp_config_json: Option<String>,
}

fn normalize_names(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut cleaned = Vec::new();
    let mut seen = HashSet::new();
    for value in values {
        let value = value.trim().to_string();
        if value.is_empty() {
            continue;
        }
        if seen.insert(value.clone()) {
            cleaned.push(value);
        }
    }
    cleaned
}

impl AgentRuntime {
    fn session_allowed_tools_key(session_id: &str) -> String {
        format!("{SESSION_ALLOWED_TOOLS_KEY_PREFIX}{session_id}")
    }

    fn session_mcp_config_key(session_id: &str) -> String {
        format!("{SESSION_MCP_CONFIG_KEY_PREFIX}{session_id}")
    }

    fn session_allowed_skills_key(session_id: &str) -> String {
        format!("{SESSION_ALLOWED_SKILLS_KEY_PREFIX}{session_id}")
    }

    fn session_skill_policy_key(session_id: &str) -> String {
        format!("{SESSION_SKILL_POLICY_KEY_PREFIX}{session_id}")
    }

    pub fn load_session_policy_snapshot(
        &self,
        session_id: &str,
    ) -> anyhow::Result<SessionPolicySnapshot> {
        Ok(SessionPolicySnapshot {
            allowed_tools: self.load_session_allowed_tools(session_id)?,
            allowed_skills: self.load_session_allowed_skills(session_id)?,
            skill_policy: self.load_session_skill_policy(session_id)?,
            mcp_config_json: self.load_session_mcp_config(session_id)?,
        })
    }

    pub fn configure_session_tooling(
        &self,
        session_id: &str,
        allowed_tools: Option<Vec<String>>,
        mcp_config_json: Option<String>,
    ) -> anyhow::Result<()> {
        if let Some(tools) = allowed_tools {
            self.set_session_allowed_tools(session_id, tools)?;
        }
        if let Some(config_json) = mcp_config_json {
            self.set_session_mcp_config(session_id, config_json)?;
        }
        Ok(())
    }

    pub fn set_session_allowed_tools(
        &self,
        session_id: &str,
        tools: Vec<String>,
    ) -> anyhow::Result<()> {
        let raw = serde_json::to_string(&normalize_names(tools))
            .context("serialize session allowed tools")?;
        self.memory
            .kv_set(&Self::session_allowed_tools_key(session_id), &raw)
            .context("kv_set session allowed tools")?;
        Ok(())
    }

    pub fn set_session_mcp_config(&self, session_id: &str, raw_json: String) -> anyhow::Result<()> {
        let raw_json = raw_json.trim().to_string();
        self.memory
            .kv_set(&Self::session_mcp_config_key(session_id), &raw_json)
            .context("kv_set session mcp config")?;
        Ok(())
    }

    pub(crate) fn load_session_mcp_config(
        &self,
        session_id: &str,
    ) -> anyhow::Result<Option<String>> {
        let raw = self
            .memory
            .kv_get(&Self::session_mcp_config_key(session_id))
            .context("kv_get session mcp config")?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        let raw = raw.trim().to_string();
        if raw.is_empty() {
            return Ok(None);
        }
        Ok(Some(raw))
    }

    pub(crate) fn load_session_allowed_tools(
        &self,
        session_id: &str,
    ) -> anyhow::Result<Option<Vec<String>>> {
        let raw = self
            .memory
            .kv_get(&Self::session_allowed_tools_key(session_id))
            .context("kv_get session allowed tools")?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        let parsed: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();
        let cleaned = normalize_names(parsed);
        if cleaned.is_empty() {
            return Ok(None);
        }
        Ok(Some(cleaned))
    }

    pub fn set_session_allowed_skills(
        &self,
        session_id: &str,
        skills: Vec<String>,
    ) -> anyhow::Result<()> {
        let raw = serde_json::to_string(&normalize_names(skills))
            .context("serialize session allowed skills")?;
        self.memory
            .kv_set(&Self::session_allowed_skills_key(session_id), &raw)
            .context("kv_set session allowed skills")?;
        Ok(())
    }

    pub(crate) fn load_session_allowed_skills(
        &self,
        session_id: &str,
    ) -> anyhow::Result<Option<Vec<String>>> {
        let raw = self
            .memory
            .kv_get(&Self::session_allowed_skills_key(session_id))
            .context("kv_get session allowed skills")?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        let parsed: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();
        let cleaned = normalize_names(parsed);
        if cleaned.is_empty() {
            return Ok(None);
        }
        Ok(Some(cleaned))
    }

    pub fn set_session_skill_policy(
        &self,
        session_id: &str,
        policy: SessionSkillPolicy,
    ) -> anyhow::Result<()> {
        let raw = serde_json::to_string(&policy).context("serialize session skill policy")?;
        self.memory
            .kv_set(&Self::session_skill_policy_key(session_id), &raw)
            .context("kv_set session skill policy")?;
        Ok(())
    }

    pub(crate) fn load_session_skill_policy(
        &self,
        session_id: &str,
    ) -> anyhow::Result<SessionSkillPolicy> {
        let raw = self
            .memory
            .kv_get(&Self::session_skill_policy_key(session_id))
            .context("kv_get session skill policy")?;
        let Some(raw) = raw else {
            return Ok(SessionSkillPolicy::default());
        };
        let policy: SessionSkillPolicy = serde_json::from_str(&raw).unwrap_or_default();
        Ok(policy)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use rexos_kernel::config::{LlmConfig, ProviderConfig, ProviderKind, RexosConfig, RouteConfig};
    use rexos_kernel::paths::RexosPaths;
    use rexos_kernel::router::{ModelRouter, TaskKind};
    use rexos_kernel::security::SecurityConfig;
    use rexos_llm::registry::LlmRegistry;
    use rexos_memory::MemoryStore;

    use crate::records::{WorkflowRunToolArgs, WorkflowStepToolArgs};
    use crate::AgentRuntime;

    fn build_agent(memory: MemoryStore) -> AgentRuntime {
        let mut providers = BTreeMap::new();
        providers.insert(
            "ollama".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompatible,
                base_url: "http://127.0.0.1:11434/v1".to_string(),
                api_key_env: "".to_string(),
                default_model: "x".to_string(),
                aws_bedrock: None,
            },
        );

        let security = SecurityConfig::default();
        let cfg = RexosConfig {
            llm: LlmConfig::default(),
            providers,
            router: Default::default(),
            security: security.clone(),
        };
        let llms = LlmRegistry::from_config(&cfg).unwrap();
        let router = ModelRouter::new(rexos_kernel::config::RouterConfig {
            planning: RouteConfig {
                provider: "ollama".to_string(),
                model: "x".to_string(),
            },
            coding: RouteConfig {
                provider: "ollama".to_string(),
                model: "x".to_string(),
            },
            summary: RouteConfig {
                provider: "ollama".to_string(),
                model: "x".to_string(),
            },
        });
        AgentRuntime::new_with_security_config(memory, llms, router, security)
    }

    #[test]
    fn session_policy_snapshot_round_trips_and_normalizes() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = RexosPaths {
            base_dir: tmp.path().join(".loopforge"),
        };
        paths.ensure_dirs().unwrap();

        let memory = MemoryStore::open_or_create(&paths).unwrap();
        let agent = build_agent(memory);

        agent
            .set_session_allowed_tools(
                "s1",
                vec![
                    " fs_read ".to_string(),
                    "".to_string(),
                    "fs_write".to_string(),
                    "fs_read".to_string(),
                ],
            )
            .unwrap();
        agent
            .set_session_allowed_skills(
                "s1",
                vec![
                    " safe-skill ".to_string(),
                    "safe-skill".to_string(),
                    "x".to_string(),
                    "".to_string(),
                ],
            )
            .unwrap();
        agent
            .set_session_skill_policy(
                "s1",
                crate::SessionSkillPolicy {
                    allowlist: vec!["shell-helper".to_string()],
                    require_approval: true,
                    auto_approve_readonly: false,
                },
            )
            .unwrap();
        agent
            .set_session_mcp_config("s1", "  {\"servers\":{}} ".to_string())
            .unwrap();

        let snapshot = agent.load_session_policy_snapshot("s1").unwrap();
        assert_eq!(
            snapshot.allowed_tools,
            Some(vec!["fs_read".to_string(), "fs_write".to_string()])
        );
        assert_eq!(
            snapshot.allowed_skills,
            Some(vec!["safe-skill".to_string(), "x".to_string()])
        );
        assert_eq!(
            snapshot.mcp_config_json,
            Some("{\"servers\":{}}".to_string())
        );
        assert_eq!(
            snapshot.skill_policy.allowlist,
            vec!["shell-helper".to_string()]
        );
        assert!(snapshot.skill_policy.require_approval);
        assert!(!snapshot.skill_policy.auto_approve_readonly);
    }

    #[test]
    fn session_policy_snapshot_defaults_when_missing_or_blank() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = RexosPaths {
            base_dir: tmp.path().join(".loopforge"),
        };
        paths.ensure_dirs().unwrap();

        let memory = MemoryStore::open_or_create(&paths).unwrap();
        let agent = build_agent(memory);

        agent
            .set_session_mcp_config("s2", "   ".to_string())
            .unwrap();
        let snapshot = agent.load_session_policy_snapshot("s2").unwrap();
        assert!(snapshot.allowed_tools.is_none());
        assert!(snapshot.allowed_skills.is_none());
        assert!(snapshot.mcp_config_json.is_none());
        assert!(!snapshot.skill_policy.auto_approve_readonly);
        assert!(!snapshot.skill_policy.require_approval);
        assert!(snapshot.skill_policy.allowlist.is_empty());
    }

    #[tokio::test]
    async fn session_policy_workflow_uses_allowed_tools_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace_root = tmp.path().join("workspace");
        std::fs::create_dir_all(&workspace_root).unwrap();

        let paths = RexosPaths {
            base_dir: tmp.path().join(".loopforge"),
        };
        paths.ensure_dirs().unwrap();
        let memory = MemoryStore::open_or_create(&paths).unwrap();
        let agent = build_agent(memory);

        agent
            .set_session_allowed_tools("s3", vec!["fs_read".to_string()])
            .unwrap();

        let res = agent
            .workflow_run(
                &workspace_root,
                "s3",
                TaskKind::Coding,
                WorkflowRunToolArgs {
                    workflow_id: Some("wf-policy".to_string()),
                    name: None,
                    steps: vec![WorkflowStepToolArgs {
                        tool: "fs_write".to_string(),
                        arguments: serde_json::json!({
                            "path": "x.txt",
                            "content": "blocked",
                        }),
                        name: None,
                        approval_required: None,
                    }],
                    continue_on_error: None,
                },
            )
            .await
            .unwrap();

        let res: serde_json::Value = serde_json::from_str(&res).unwrap();
        let saved_to = res["saved_to"].as_str().unwrap();
        let state_raw = std::fs::read_to_string(saved_to).unwrap();
        let state: serde_json::Value = serde_json::from_str(&state_raw).unwrap();
        let err = state["steps"][0]["error"].as_str().unwrap();
        assert!(err.contains("workflow step 0 (fs_write)"), "got: {err}");
        assert!(!workspace_root.join("x.txt").exists());
    }
}
