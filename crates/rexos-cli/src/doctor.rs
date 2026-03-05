use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use anyhow::Context;
use rexos::config::{ProviderKind, RexosConfig};
use rexos::paths::RexosPaths;
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct DoctorOptions {
    pub paths: RexosPaths,
    pub timeout: Duration,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Ok,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorCheck {
    pub id: String,
    pub status: CheckStatus,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheck>,
    pub summary: DoctorSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorSummary {
    pub ok: u32,
    pub warn: u32,
    pub error: u32,
}

impl DoctorReport {
    pub fn exit_code(&self, strict: bool) -> i32 {
        if self.summary.error > 0 {
            return 1;
        }
        if strict && self.summary.warn > 0 {
            return 1;
        }
        0
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str("LoopForge doctor\n\n");
        for c in &self.checks {
            let prefix = match c.status {
                CheckStatus::Ok => "OK  ",
                CheckStatus::Warn => "WARN",
                CheckStatus::Error => "ERR ",
            };
            if c.message.trim().is_empty() {
                out.push_str(&format!("{prefix} {id}\n", id = c.id));
            } else {
                out.push_str(&format!(
                    "{prefix} {id}: {msg}\n",
                    id = c.id,
                    msg = c.message
                ));
            }
        }
        out.push_str(&format!(
            "\nSummary: ok={} warn={} error={}\n",
            self.summary.ok, self.summary.warn, self.summary.error
        ));
        out
    }
}

pub async fn run_doctor(opts: DoctorOptions) -> anyhow::Result<DoctorReport> {
    let http = reqwest::Client::builder()
        .timeout(opts.timeout)
        .build()
        .context("build http client")?;

    let mut checks: Vec<DoctorCheck> = Vec::new();

    // Paths
    checks.push(DoctorCheck {
        id: "paths.base_dir".to_string(),
        status: CheckStatus::Ok,
        message: opts.paths.base_dir.display().to_string(),
    });

    let config_path = opts.paths.config_path();
    let db_path = opts.paths.db_path();

    checks.push(DoctorCheck {
        id: "paths.config".to_string(),
        status: if config_path.exists() {
            CheckStatus::Ok
        } else {
            CheckStatus::Warn
        },
        message: format!(
            "{}{}",
            config_path.display(),
            if config_path.exists() {
                ""
            } else {
                " (missing; run `loopforge init`)"
            }
        ),
    });

    checks.push(DoctorCheck {
        id: "paths.db".to_string(),
        status: if db_path.exists() {
            CheckStatus::Ok
        } else {
            CheckStatus::Warn
        },
        message: format!(
            "{}{}",
            db_path.display(),
            if db_path.exists() {
                ""
            } else {
                " (missing; run `loopforge init`)"
            }
        ),
    });

    // Config/provider checks.
    let cfg = if config_path.exists() {
        match RexosConfig::load(&opts.paths) {
            Ok(cfg) => {
                checks.push(DoctorCheck {
                    id: "config.parse".to_string(),
                    status: CheckStatus::Ok,
                    message: "config.toml parsed".to_string(),
                });
                Some(cfg)
            }
            Err(e) => {
                checks.push(DoctorCheck {
                    id: "config.parse".to_string(),
                    status: CheckStatus::Error,
                    message: e.to_string(),
                });
                None
            }
        }
    } else {
        None
    };

    if let Some(cfg) = cfg.as_ref() {
        // Router points to known providers?
        for (kind, route) in [
            ("planning", &cfg.router.planning),
            ("coding", &cfg.router.coding),
            ("summary", &cfg.router.summary),
        ] {
            let id = format!("router.{kind}.provider");
            if cfg.providers.contains_key(&route.provider) {
                checks.push(DoctorCheck {
                    id,
                    status: CheckStatus::Ok,
                    message: route.provider.clone(),
                });
            } else {
                checks.push(DoctorCheck {
                    id,
                    status: CheckStatus::Error,
                    message: format!(
                        "unknown provider '{}' (defined: [{}])",
                        route.provider,
                        cfg.providers.keys().cloned().collect::<Vec<_>>().join(", ")
                    ),
                });
            }
        }

        // API keys present?
        let mut missing: Vec<String> = Vec::new();
        for (name, p) in &cfg.providers {
            if !p.api_key_env.trim().is_empty() && std::env::var(&p.api_key_env).is_err() {
                missing.push(format!("{name} -> {}", p.api_key_env));
            }
        }
        if missing.is_empty() {
            checks.push(DoctorCheck {
                id: "providers.api_keys".to_string(),
                status: CheckStatus::Ok,
                message: "all required provider env vars are set".to_string(),
            });
        } else {
            checks.push(DoctorCheck {
                id: "providers.api_keys".to_string(),
                status: CheckStatus::Warn,
                message: format!(
                    "missing env vars: {}",
                    missing.into_iter().take(8).collect::<Vec<_>>().join(", ")
                ),
            });
        }

        // Probe Ollama only when it looks local and requires no key.
        if let Some(ollama) = cfg.providers.get("ollama") {
            if ollama.kind == ProviderKind::OpenAiCompatible && ollama.api_key_env.trim().is_empty()
            {
                if let Ok(url) = reqwest::Url::parse(&ollama.base_url) {
                    let is_loopback = matches!(
                        url.host_str(),
                        Some("127.0.0.1") | Some("localhost") | Some("::1")
                    );
                    if is_loopback {
                        let probe = format!("{}/models", ollama.base_url.trim_end_matches('/'));
                        let res = http.get(&probe).send().await;
                        match res {
                            Ok(r) if r.status().is_success() => checks.push(DoctorCheck {
                                id: "ollama.http".to_string(),
                                status: CheckStatus::Ok,
                                message: format!("GET {probe} -> {}", r.status()),
                            }),
                            Ok(r) => checks.push(DoctorCheck {
                                id: "ollama.http".to_string(),
                                status: CheckStatus::Warn,
                                message: format!("GET {probe} -> {}", r.status()),
                            }),
                            Err(e) => checks.push(DoctorCheck {
                                id: "ollama.http".to_string(),
                                status: CheckStatus::Warn,
                                message: format!("GET {probe} failed: {e}"),
                            }),
                        }
                    }
                }
            }
        }
    }

    // Browser checks.
    if let Ok(cdp) = std::env::var("REXOS_BROWSER_CDP_HTTP") {
        let cdp = cdp.trim().to_string();
        if !cdp.is_empty() {
            let probe = format!("{}/json/version", cdp.trim_end_matches('/'));
            let res = http.get(&probe).send().await;
            match res {
                Ok(r) if r.status().is_success() => checks.push(DoctorCheck {
                    id: "browser.cdp_http".to_string(),
                    status: CheckStatus::Ok,
                    message: format!("GET {probe} -> {}", r.status()),
                }),
                Ok(r) => checks.push(DoctorCheck {
                    id: "browser.cdp_http".to_string(),
                    status: CheckStatus::Warn,
                    message: format!("GET {probe} -> {}", r.status()),
                }),
                Err(e) => checks.push(DoctorCheck {
                    id: "browser.cdp_http".to_string(),
                    status: CheckStatus::Warn,
                    message: format!("GET {probe} failed: {e}"),
                }),
            }
        }
    } else {
        let discovered = discover_chromium_executable();
        match discovered {
            Some(p) => checks.push(DoctorCheck {
                id: "browser.chromium".to_string(),
                status: CheckStatus::Ok,
                message: p.display().to_string(),
            }),
            None => checks.push(DoctorCheck {
                id: "browser.chromium".to_string(),
                status: CheckStatus::Warn,
                message: "chromium/chrome/edge not found; install a Chromium-based browser or set REXOS_BROWSER_CHROME_PATH (or use REXOS_BROWSER_CDP_HTTP)".to_string(),
            }),
        }
    }

    // Tooling checks.
    checks.push(check_command("git", &["--version"], "tools.git", true));
    checks.push(check_command(
        "docker",
        &["--version"],
        "tools.docker",
        false,
    ));

    let summary = summarize(&checks);
    Ok(DoctorReport { checks, summary })
}

fn summarize(checks: &[DoctorCheck]) -> DoctorSummary {
    let mut ok = 0u32;
    let mut warn = 0u32;
    let mut error = 0u32;
    for c in checks {
        match c.status {
            CheckStatus::Ok => ok += 1,
            CheckStatus::Warn => warn += 1,
            CheckStatus::Error => error += 1,
        }
    }
    DoctorSummary { ok, warn, error }
}

fn check_command(command: &str, args: &[&str], id: &str, required: bool) -> DoctorCheck {
    let output = Command::new(command).args(args).output();
    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            DoctorCheck {
                id: id.to_string(),
                status: CheckStatus::Ok,
                message: if stdout.is_empty() {
                    format!("{command} available")
                } else {
                    stdout
                },
            }
        }
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let mut msg = format!("{command} returned non-zero");
            if !stdout.is_empty() {
                msg.push_str(&format!("; stdout={stdout}"));
            }
            if !stderr.is_empty() {
                msg.push_str(&format!("; stderr={stderr}"));
            }
            DoctorCheck {
                id: id.to_string(),
                status: if required {
                    CheckStatus::Error
                } else {
                    CheckStatus::Warn
                },
                message: msg,
            }
        }
        Err(e) => DoctorCheck {
            id: id.to_string(),
            status: if required {
                CheckStatus::Error
            } else {
                CheckStatus::Warn
            },
            message: format!("{command} not found ({e})"),
        },
    }
}

fn discover_chromium_executable() -> Option<PathBuf> {
    if let Ok(v) = std::env::var("REXOS_BROWSER_CHROME_PATH") {
        let p = PathBuf::from(v);
        if p.exists() {
            return Some(p);
        }
    }

    if let Ok(path) = std::env::var("PATH") {
        let mut names: Vec<&str> = vec![
            "google-chrome",
            "chrome",
            "chromium",
            "chromium-browser",
            "microsoft-edge",
            "msedge",
            "brave",
            "brave-browser",
        ];
        if cfg!(windows) {
            names = vec!["chrome.exe", "msedge.exe", "brave.exe", "chromium.exe"];
        }

        for dir in std::env::split_paths(&path) {
            for name in &names {
                let candidate = dir.join(name);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }

    if cfg!(windows) {
        // Common install locations.
        let mut candidates: Vec<PathBuf> = Vec::new();
        for key in ["ProgramFiles", "ProgramFiles(x86)", "LocalAppData"] {
            if let Ok(base) = std::env::var(key) {
                let base = PathBuf::from(base);
                candidates.push(base.join("Google/Chrome/Application/chrome.exe"));
                candidates.push(base.join("Microsoft/Edge/Application/msedge.exe"));
                candidates.push(base.join("BraveSoftware/Brave-Browser/Application/brave.exe"));
            }
        }
        for c in candidates {
            if c.exists() {
                return Some(c);
            }
        }
    } else if cfg!(target_os = "macos") {
        let candidates = [
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
            "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
        ];
        for c in candidates {
            let p = PathBuf::from(c);
            if p.exists() {
                return Some(p);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;
    use axum::{Json, Router};
    use serde_json::json;

    #[tokio::test]
    async fn doctor_probes_local_ollama_models_and_cdp_version() {
        async fn models() -> Json<serde_json::Value> {
            Json(json!({ "data": [] }))
        }
        async fn cdp_version() -> Json<serde_json::Value> {
            Json(json!({ "Browser": "Chrome/1.0" }))
        }

        let app = Router::new()
            .route("/v1/models", get(models))
            .route("/json/version", get(cdp_version));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let tmp = tempfile::tempdir().unwrap();
        let paths = RexosPaths {
            base_dir: tmp.path().join(".rexos"),
        };
        std::fs::create_dir_all(&paths.base_dir).unwrap();

        let cfg = RexosConfig {
            llm: rexos::config::LlmConfig::default(),
            providers: [(
                "ollama".to_string(),
                rexos::config::ProviderConfig {
                    kind: ProviderKind::OpenAiCompatible,
                    base_url: format!("http://{addr}/v1"),
                    api_key_env: "".to_string(),
                    default_model: "x".to_string(),
                },
            )]
            .into_iter()
            .collect(),
            router: rexos::config::RouterConfig::default(),
        };
        std::fs::write(paths.config_path(), toml::to_string(&cfg).unwrap()).unwrap();
        std::env::set_var("REXOS_BROWSER_CDP_HTTP", format!("http://{addr}"));

        let report = run_doctor(DoctorOptions {
            paths,
            timeout: Duration::from_millis(500),
        })
        .await
        .unwrap();

        let statuses: std::collections::BTreeMap<String, CheckStatus> = report
            .checks
            .iter()
            .map(|c| (c.id.clone(), c.status))
            .collect();
        assert_eq!(statuses.get("ollama.http"), Some(&CheckStatus::Ok));
        assert_eq!(statuses.get("browser.cdp_http"), Some(&CheckStatus::Ok));

        std::env::remove_var("REXOS_BROWSER_CDP_HTTP");
        server.abort();
    }
}
