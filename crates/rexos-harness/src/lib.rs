use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context};

const FEATURES_JSON: &str = "features.json";
const PROGRESS_MD: &str = "rexos-progress.md";
const INIT_SH: &str = "init.sh";
const INIT_PS1: &str = "init.ps1";
const REXOS_DIR: &str = ".rexos";
const SESSION_ID_FILE: &str = "session_id";

pub fn init_workspace(workspace_dir: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(workspace_dir)
        .with_context(|| format!("create workspace dir: {}", workspace_dir.display()))?;

    let features_path = workspace_dir.join(FEATURES_JSON);
    let progress_path = workspace_dir.join(PROGRESS_MD);
    let init_sh_path = workspace_dir.join(INIT_SH);
    let init_ps1_path = workspace_dir.join(INIT_PS1);

    if features_path.exists() || progress_path.exists() || init_sh_path.exists() || init_ps1_path.exists() {
        bail!("workspace already initialized");
    }

    let features_json = serde_json::json!({
        "version": 1,
        "updated_at": "",
        "rules": {
            "editing": "Only change `passes` (false -> true) and optionally `notes`. Do not delete or reorder items.",
            "completion": "A feature can only be marked passing after required tests/smoke checks are run."
        },
        "features": []
    });
    std::fs::write(&features_path, serde_json::to_string_pretty(&features_json)?)
        .with_context(|| format!("write {}", features_path.display()))?;

    std::fs::write(
        &progress_path,
        "# RexOS Progress Log\n\nThis file is append-only.\n",
    )
    .with_context(|| format!("write {}", progress_path.display()))?;

    std::fs::write(
        &init_sh_path,
        r#"#!/usr/bin/env bash
set -euo pipefail

echo "[rexos] init.sh: customize this script for your project"
"#,
    )
    .with_context(|| format!("write {}", init_sh_path.display()))?;

    std::fs::write(
        &init_ps1_path,
        r#"$ErrorActionPreference = "Stop"

Write-Output "[rexos] init.ps1: customize this script for your project"
"#,
    )
    .with_context(|| format!("write {}", init_ps1_path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&init_sh_path)?.permissions();
        perms.set_mode(perms.mode() | 0o111);
        std::fs::set_permissions(&init_sh_path, perms)?;
    }

    ensure_git_repo(workspace_dir)?;
    git(workspace_dir, ["add", FEATURES_JSON, PROGRESS_MD, INIT_SH, INIT_PS1])?;
    git_with_identity(
        workspace_dir,
        [
            "commit",
            "-m",
            "chore: initialize rexos harness",
            "--no-gpg-sign",
        ],
    )?;

    Ok(())
}

pub fn resolve_session_id(workspace_dir: &Path) -> anyhow::Result<String> {
    let rexos_dir = workspace_dir.join(REXOS_DIR);
    std::fs::create_dir_all(&rexos_dir)
        .with_context(|| format!("create {}", rexos_dir.display()))?;

    ensure_gitignore_has_rexos_dir(workspace_dir)?;

    let path = rexos_dir.join(SESSION_ID_FILE);
    if path.exists() {
        let raw =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let s = raw.trim().to_string();
        if s.is_empty() {
            bail!("session_id file is empty");
        }
        return Ok(s);
    }

    let id = uuid::Uuid::new_v4().to_string();
    std::fs::write(&path, format!("{id}\n")).with_context(|| format!("write {}", path.display()))?;
    Ok(id)
}

pub async fn bootstrap_with_prompt(
    agent: &rexos_runtime::AgentRuntime,
    workspace_dir: &Path,
    session_id: &str,
    prompt: &str,
) -> anyhow::Result<()> {
    if !is_initialized(workspace_dir) {
        init_workspace(workspace_dir)?;
    }

    preflight(workspace_dir)?;

    let initializer_system = initializer_system_prompt();
    let _ = agent
        .run_session(
            workspace_dir.to_path_buf(),
            session_id,
            Some(initializer_system),
            prompt,
            rexos_kernel::router::TaskKind::Coding,
        )
        .await?;

    run_init_script(workspace_dir)?;
    commit_checkpoint_if_dirty(workspace_dir, "chore: rexos harness bootstrap")?;
    Ok(())
}

pub async fn run_harness(
    agent: &rexos_runtime::AgentRuntime,
    workspace_dir: &Path,
    session_id: &str,
    user_prompt: &str,
    max_attempts: usize,
) -> anyhow::Result<String> {
    preflight(workspace_dir)?;

    let harness_system = coding_system_prompt();
    let mut prompt = user_prompt.to_string();

    for attempt in 1..=max_attempts.max(1) {
        let out = agent
            .run_session(
                workspace_dir.to_path_buf(),
                session_id,
                Some(harness_system),
                &prompt,
                rexos_kernel::router::TaskKind::Coding,
            )
            .await?;

        match run_init_script_capture(workspace_dir) {
            Ok(_) => {
                commit_checkpoint_if_dirty(workspace_dir, "chore: rexos harness checkpoint")?;
                return Ok(out);
            }
            Err(e) => {
                if attempt >= max_attempts.max(1) {
                    return Err(e);
                }
                prompt = format!(
                    "init.sh failed after your changes.\n\nOutput:\n{}\n\nFix the issues and make `./init.sh` pass.",
                    e
                );
            }
        }
    }

    bail!("unreachable: harness loop exhausted")
}

pub fn preflight(workspace_dir: &Path) -> anyhow::Result<()> {
    let features_path = workspace_dir.join(FEATURES_JSON);
    let progress_path = workspace_dir.join(PROGRESS_MD);
    let init_sh_path = workspace_dir.join(INIT_SH);
    let init_ps1_path = workspace_dir.join(INIT_PS1);

    if !features_path.exists()
        || !progress_path.exists()
        || (!init_sh_path.exists() && !init_ps1_path.exists())
    {
        bail!(
            "workspace not initialized; run `rexos harness init {}`",
            workspace_dir.display()
        );
    }

    println!("[rexos] workspace: {}", workspace_dir.display());

    let git_log = Command::new("git")
        .args(["--no-pager", "log", "-5", "--oneline"])
        .current_dir(workspace_dir)
        .output()
        .with_context(|| format!("run git log in {}", workspace_dir.display()))?;
    if git_log.status.success() {
        let s = String::from_utf8_lossy(&git_log.stdout);
        println!("[rexos] recent commits:\n{s}");
    } else {
        let e = String::from_utf8_lossy(&git_log.stderr);
        println!("[rexos] git log failed (continuing):\n{e}");
    }

    let progress = std::fs::read_to_string(&progress_path)
        .with_context(|| format!("read {}", progress_path.display()))?;
    println!(
        "[rexos] progress (tail):\n{}",
        tail_lines(&progress, 20).join("\n")
    );

    let features_raw = std::fs::read_to_string(&features_path)
        .with_context(|| format!("read {}", features_path.display()))?;
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&features_raw) {
        if let Some(first) = first_failing_feature(&v) {
            println!("[rexos] next feature: {first}");
        } else {
            println!("[rexos] next feature: (none pending)");
        }
    } else {
        println!("[rexos] features.json: could not parse (continuing)");
    }

    run_init_script(workspace_dir)?;

    Ok(())
}

fn ensure_git_repo(workspace_dir: &Path) -> anyhow::Result<()> {
    if workspace_dir.join(".git").exists() {
        return Ok(());
    }

    if git(workspace_dir, ["init", "-b", "main"]).is_ok() {
        return Ok(());
    }

    git(workspace_dir, ["init"])?;
    git(workspace_dir, ["checkout", "-b", "main"])?;
    Ok(())
}

fn git<const N: usize>(workspace_dir: &Path, args: [&str; N]) -> anyhow::Result<()> {
    let output = Command::new("git")
        .args(args)
        .current_dir(workspace_dir)
        .output()
        .with_context(|| format!("run git {:?}", args))?;

    if !output.status.success() {
        bail!(
            "git failed: {:?}\nstdout:\n{}\nstderr:\n{}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn git_with_identity<const N: usize>(
    workspace_dir: &Path,
    args: [&str; N],
) -> anyhow::Result<()> {
    let output = Command::new("git")
        .arg("-c")
        .arg("user.name=RexOS")
        .arg("-c")
        .arg("user.email=rexos@localhost")
        .args(args)
        .current_dir(workspace_dir)
        .output()
        .with_context(|| format!("run git (identity) {:?}", args))?;

    if !output.status.success() {
        bail!(
            "git failed: {:?}\nstdout:\n{}\nstderr:\n{}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn tail_lines(s: &str, n: usize) -> Vec<&str> {
    let mut lines: Vec<&str> = s.lines().collect();
    if lines.len() > n {
        lines.drain(0..lines.len() - n);
    }
    lines
}

fn is_initialized(workspace_dir: &Path) -> bool {
    workspace_dir.join(FEATURES_JSON).exists()
        && workspace_dir.join(PROGRESS_MD).exists()
        && workspace_dir.join(INIT_SH).exists()
}

fn ensure_gitignore_has_rexos_dir(workspace_dir: &Path) -> anyhow::Result<()> {
    if !workspace_dir.join(".git").exists() {
        return Ok(());
    }

    let path = workspace_dir.join(".gitignore");
    let line = ".rexos/";

    let mut content = if path.exists() {
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?
    } else {
        String::new()
    };

    if content.lines().any(|l| l.trim() == line) {
        return Ok(());
    }

    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(line);
    content.push('\n');

    std::fs::write(&path, content).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn initializer_system_prompt() -> &'static str {
    r#"You are RexOS initializer.

Your job:
- Generate a comprehensive `features.json` from the user prompt.
- Keep `features.json` as a stable checklist. Do NOT delete or reorder items after creation.
- Each feature must include: id, description, steps, passes=false, and optional notes.
- Update the workspace init script(s) (`init.sh`, and `init.ps1` on Windows) to run the minimal smoke checks/tests required to verify features.
- Append a short entry to `rexos-progress.md` describing what you initialized.

Rules:
- Work only inside the workspace directory.
- Prefer tools (`fs_read`, `fs_write`, `shell`) to inspect and change files.
- After edits, run the workspace init script (`./init.sh`, or `./init.ps1` on Windows) and ensure it succeeds.
- Commit your changes to git with a descriptive message.
"#
}

fn coding_system_prompt() -> &'static str {
    r#"You are RexOS running a long-horizon harness coding session.

Rules:
- Work only inside the workspace directory.
- Make small, incremental progress (one feature at a time).
- Prefer using tools (`fs_read`, `fs_write`, `shell`) to inspect and change files.
- If you change code, run the workspace init script (smoke checks) and fix any failures.
- If both `init.sh` and `init.ps1` exist, keep them functionally equivalent.
- Append a short summary to `rexos-progress.md`.
- Commit meaningful progress to git with a descriptive message.
"#
}

#[derive(Debug, Clone, Copy)]
enum InitScript {
    Bash,
    PowerShell,
}

fn select_init_script(workspace_dir: &Path) -> anyhow::Result<InitScript> {
    let sh_exists = workspace_dir.join(INIT_SH).exists();
    let ps1_exists = workspace_dir.join(INIT_PS1).exists();

    if cfg!(windows) {
        if ps1_exists {
            return Ok(InitScript::PowerShell);
        }
        if sh_exists {
            return Ok(InitScript::Bash);
        }
    } else if sh_exists {
        return Ok(InitScript::Bash);
    }

    if ps1_exists && !sh_exists {
        bail!("init.ps1 exists but init.sh is missing");
    }

    bail!("no init script found (expected init.sh and/or init.ps1)");
}

fn run_init_script(workspace_dir: &Path) -> anyhow::Result<()> {
    match select_init_script(workspace_dir)? {
        InitScript::Bash => {
            let status = Command::new("bash")
                .arg(INIT_SH)
                .current_dir(workspace_dir)
                .status()
                .with_context(|| format!("run {}", workspace_dir.join(INIT_SH).display()))?;
            if !status.success() {
                bail!("init.sh failed");
            }
            Ok(())
        }
        InitScript::PowerShell => {
            let status = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                    INIT_PS1,
                ])
                .current_dir(workspace_dir)
                .status()
                .with_context(|| format!("run {}", workspace_dir.join(INIT_PS1).display()))?;
            if !status.success() {
                bail!("init.ps1 failed");
            }
            Ok(())
        }
    }
}

fn run_init_script_capture(workspace_dir: &Path) -> anyhow::Result<String> {
    let output = match select_init_script(workspace_dir)? {
        InitScript::Bash => Command::new("bash")
            .arg(INIT_SH)
            .current_dir(workspace_dir)
            .output()
            .with_context(|| format!("run {}", workspace_dir.join(INIT_SH).display()))?,
        InitScript::PowerShell => Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                INIT_PS1,
            ])
            .current_dir(workspace_dir)
            .output()
            .with_context(|| format!("run {}", workspace_dir.join(INIT_PS1).display()))?,
    };

    let mut combined = String::new();
    combined.push_str(&String::from_utf8_lossy(&output.stdout));
    combined.push_str(&String::from_utf8_lossy(&output.stderr));

    if !output.status.success() {
        bail!("init failed: {}", combined.trim());
    }

    Ok(combined)
}

fn commit_checkpoint_if_dirty(workspace_dir: &Path, message: &str) -> anyhow::Result<()> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(workspace_dir)
        .output()
        .context("git status")?;

    if !output.status.success() {
        bail!(
            "git status failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    if output.stdout.is_empty() {
        return Ok(());
    }

    git(workspace_dir, ["add", "-A"])?;
    git_with_identity(
        workspace_dir,
        ["commit", "-m", message, "--no-gpg-sign"],
    )?;
    Ok(())
}

fn first_failing_feature(v: &serde_json::Value) -> Option<String> {
    let arr = v.get("features")?.as_array()?;
    for f in arr {
        if f.get("passes").and_then(|p| p.as_bool()) == Some(false) {
            let id = f.get("id").and_then(|x| x.as_str()).unwrap_or("<no id>");
            let desc = f
                .get("description")
                .and_then(|x| x.as_str())
                .unwrap_or("");
            return Some(format!("{id} - {desc}"));
        }
    }
    None
}
