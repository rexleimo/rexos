use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApprovalMode {
    Off,
    Warn,
    Enforce,
}

impl ApprovalMode {
    pub(crate) fn from_env() -> Self {
        let raw = std::env::var("LOOPFORGE_APPROVAL_MODE")
            .unwrap_or_else(|_| "off".to_string())
            .to_lowercase();
        match raw.as_str() {
            "warn" => Self::Warn,
            "enforce" => Self::Enforce,
            _ => Self::Off,
        }
    }
}

pub(crate) fn tool_requires_approval(
    name: &str,
    arguments_json: &str,
    explicit_gate: bool,
) -> bool {
    if explicit_gate {
        return true;
    }

    match name {
        "shell" | "docker_exec" | "process_start" => true,
        "web_fetch" | "browser_navigate" => json_bool_field(arguments_json, "allow_private"),
        _ => false,
    }
}

fn json_bool_field(arguments_json: &str, field: &str) -> bool {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(arguments_json) else {
        return false;
    };
    v.get(field).and_then(|v| v.as_bool()).unwrap_or(false)
}

pub(crate) fn tool_approval_is_granted(tool_name: &str) -> bool {
    let raw = std::env::var("LOOPFORGE_APPROVAL_ALLOW").unwrap_or_default();
    let items: HashSet<String> = raw
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    if items.contains("all") {
        return true;
    }
    items.contains(&tool_name.to_lowercase())
}

pub(crate) fn skill_approval_is_granted(skill_name: &str) -> bool {
    let raw = std::env::var("LOOPFORGE_SKILL_APPROVAL_ALLOW").unwrap_or_default();
    let items: HashSet<String> = raw
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    if items.contains("all") {
        return true;
    }
    items.contains(&skill_name.to_lowercase())
}

pub(crate) fn skill_permissions_are_readonly(permissions: &[String]) -> bool {
    if permissions.is_empty() {
        return true;
    }

    for raw in permissions {
        let p = raw.trim().to_ascii_lowercase();
        if p.is_empty() {
            continue;
        }
        if p == "readonly" {
            continue;
        }
        if p.starts_with("tool:") {
            let tool = p.trim_start_matches("tool:");
            let dangerous = [
                "shell",
                "docker_exec",
                "fs_write",
                "apply_patch",
                "process_start",
                "browser_navigate",
                "web_fetch",
            ];
            if dangerous.contains(&tool) {
                return false;
            }
            continue;
        }
        if p.contains("write")
            || p.contains("patch")
            || p.contains("delete")
            || p.contains("shell")
            || p.contains("docker")
            || p.contains("network")
            || p.contains("process")
        {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::{skill_permissions_are_readonly, tool_requires_approval};

    #[test]
    fn readonly_permissions_ignore_safe_entries() {
        let permissions = vec!["readonly".to_string(), "tool:file_read".to_string()];
        assert!(skill_permissions_are_readonly(&permissions));
    }

    #[test]
    fn readonly_permissions_reject_write_like_entries() {
        let permissions = vec!["tool:apply_patch".to_string()];
        assert!(!skill_permissions_are_readonly(&permissions));
    }

    #[test]
    fn approval_is_required_for_risky_tools_or_private_network_access() {
        assert!(tool_requires_approval("shell", "{}", false));
        assert!(tool_requires_approval(
            "browser_navigate",
            r#"{"allow_private":true}"#,
            false,
        ));
        assert!(!tool_requires_approval(
            "browser_navigate",
            r#"{"allow_private":false}"#,
            false
        ));
    }
}
