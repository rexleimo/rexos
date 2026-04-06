use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SecretMode {
    #[default]
    EnvFirst,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum LeakMode {
    #[default]
    Off,
    Warn,
    Redact,
    Enforce,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SecurityConfig {
    pub secrets: SecretsConfig,
    pub leaks: LeakGuardConfig,
    pub egress: EgressConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SecretsConfig {
    pub mode: SecretMode,
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self {
            mode: SecretMode::EnvFirst,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LeakGuardConfig {
    pub mode: LeakMode,
}

impl Default for LeakGuardConfig {
    fn default() -> Self {
        Self {
            mode: LeakMode::Off,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct EgressConfig {
    pub rules: Vec<EgressRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct EgressRule {
    pub tool: String,
    pub host: String,
    pub path_prefix: String,
    pub methods: Vec<String>,
}
