use std::path::PathBuf;

use crate::skills;

pub(super) fn run_doctor(workspace: PathBuf, json: bool, strict: bool) -> anyhow::Result<()> {
    let report = skills::doctor(&workspace)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&build_skills_doctor_json(&report)?)?
        );
    } else {
        println!("discovered_skills: {}", report.discovered_count);
        if report.issues.is_empty() {
            println!("doctor: ok");
        } else {
            for issue in &report.issues {
                let level = match issue.level {
                    skills::SkillsDoctorLevel::Warn => "warn",
                    skills::SkillsDoctorLevel::Error => "error",
                };
                if let Some(path) = &issue.path {
                    println!("[{level}] {}: {} ({path})", issue.id, issue.message);
                } else {
                    println!("[{level}] {}: {}", issue.id, issue.message);
                }
            }
        }
    }

    let has_error = report
        .issues
        .iter()
        .any(|issue| matches!(issue.level, skills::SkillsDoctorLevel::Error));
    let has_warn = report
        .issues
        .iter()
        .any(|issue| matches!(issue.level, skills::SkillsDoctorLevel::Warn));
    if has_error || (strict && has_warn) {
        std::process::exit(1);
    }
    Ok(())
}

fn build_skills_doctor_json(
    report: &skills::SkillsDoctorReport,
) -> anyhow::Result<serde_json::Value> {
    Ok(serde_json::to_value(report)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::{SkillsDoctorIssue, SkillsDoctorLevel, SkillsDoctorReport};
    use serde_json::json;

    #[test]
    fn build_skills_doctor_json_keeps_expected_shape_and_omits_missing_paths() {
        let report = SkillsDoctorReport {
            ok: false,
            discovered_count: 2,
            issues: vec![
                SkillsDoctorIssue {
                    level: SkillsDoctorLevel::Warn,
                    id: "warn.test".to_string(),
                    message: "warning".to_string(),
                    path: None,
                },
                SkillsDoctorIssue {
                    level: SkillsDoctorLevel::Error,
                    id: "error.test".to_string(),
                    message: "error".to_string(),
                    path: Some("/tmp/skill.toml".to_string()),
                },
            ],
        };

        let out = build_skills_doctor_json(&report).unwrap();
        assert_eq!(
            out,
            json!({
                "ok": false,
                "discovered_count": 2,
                "issues": [
                    { "level": "warn", "id": "warn.test", "message": "warning" },
                    { "level": "error", "id": "error.test", "message": "error", "path": "/tmp/skill.toml" },
                ]
            })
        );
    }
}
