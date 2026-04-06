use crate::{
    cli::ReleaseCommand,
    release_check::{format_release_check_report, run_release_check},
};

pub(super) fn run(command: ReleaseCommand) -> anyhow::Result<()> {
    match command {
        ReleaseCommand::Check {
            tag,
            repo_root,
            run_tests,
            json,
        } => {
            let report = run_release_check(&repo_root, tag.as_deref(), run_tests)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&build_release_check_json(&report)?)?
                );
            } else {
                println!("{}", format_release_check_report(&report));
            }
            if !report.ok {
                std::process::exit(1);
            }
            Ok(())
        }
    }
}

fn build_release_check_json(
    report: &crate::release_check::ReleaseCheckReport,
) -> anyhow::Result<serde_json::Value> {
    Ok(serde_json::to_value(report)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::release_check::{ReleaseCheckItem, ReleaseCheckReport};
    use serde_json::json;

    #[test]
    fn build_release_check_json_keeps_expected_shape() {
        let report = ReleaseCheckReport {
            ok: false,
            tag: "v1.2.3".to_string(),
            checks: vec![
                ReleaseCheckItem {
                    id: "alpha".to_string(),
                    ok: true,
                    message: "ok".to_string(),
                },
                ReleaseCheckItem {
                    id: "beta".to_string(),
                    ok: false,
                    message: "missing".to_string(),
                },
            ],
        };

        let out = build_release_check_json(&report).unwrap();
        assert_eq!(
            out,
            json!({
                "ok": false,
                "tag": "v1.2.3",
                "checks": [
                    { "id": "alpha", "ok": true, "message": "ok" },
                    { "id": "beta", "ok": false, "message": "missing" },
                ],
            })
        );
    }
}
