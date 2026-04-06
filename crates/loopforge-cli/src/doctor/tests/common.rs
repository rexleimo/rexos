use std::collections::BTreeMap;
use std::time::Duration;

use crate::doctor::{run_doctor, CheckStatus, DoctorOptions, DoctorReport};
use rexos::config::RexosConfig;
use rexos::paths::RexosPaths;

pub(super) fn test_paths() -> (tempfile::TempDir, RexosPaths) {
    let tmp = tempfile::tempdir().unwrap();
    let paths = RexosPaths {
        base_dir: tmp.path().join(".loopforge"),
    };
    std::fs::create_dir_all(&paths.base_dir).unwrap();
    (tmp, paths)
}

pub(super) fn write_config(paths: &RexosPaths, cfg: &RexosConfig) {
    std::fs::write(paths.config_path(), toml::to_string(cfg).unwrap()).unwrap();
}

pub(super) async fn run_doctor_with_timeout(paths: RexosPaths, timeout_ms: u64) -> DoctorReport {
    run_doctor(DoctorOptions {
        paths,
        timeout: Duration::from_millis(timeout_ms),
    })
    .await
    .unwrap()
}

pub(super) fn status_map(report: &DoctorReport) -> BTreeMap<String, CheckStatus> {
    report
        .checks
        .iter()
        .map(|check| (check.id.clone(), check.status))
        .collect()
}
