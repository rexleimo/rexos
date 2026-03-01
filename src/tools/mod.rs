use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, Context};

use crate::llm::openai_compat::{ToolDefinition, ToolFunctionDefinition};

#[derive(Debug, Clone)]
pub struct Toolset {
    workspace_root: PathBuf,
}

impl Toolset {
    pub fn new(workspace_root: PathBuf) -> anyhow::Result<Self> {
        let workspace_root = workspace_root
            .canonicalize()
            .with_context(|| format!("canonicalize workspace root: {}", workspace_root.display()))?;
        Ok(Self { workspace_root })
    }

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        vec![fs_read_def(), fs_write_def(), shell_def()]
    }

    pub async fn call(&self, name: &str, arguments_json: &str) -> anyhow::Result<String> {
        match name {
            "fs_read" => {
                let args: FsReadArgs = serde_json::from_str(arguments_json)
                    .context("parse fs_read arguments")?;
                self.fs_read(&args.path)
            }
            "fs_write" => {
                let args: FsWriteArgs = serde_json::from_str(arguments_json)
                    .context("parse fs_write arguments")?;
                self.fs_write(&args.path, &args.content)
            }
            "shell" => {
                let args: ShellArgs = serde_json::from_str(arguments_json)
                    .context("parse shell arguments")?;
                self.shell(&args.command, args.timeout_ms).await
            }
            _ => bail!("unknown tool: {name}"),
        }
    }

    fn fs_read(&self, user_path: &str) -> anyhow::Result<String> {
        let path = self.resolve_workspace_path(user_path)?;

        let meta = std::fs::metadata(&path)
            .with_context(|| format!("stat {}", path.display()))?;
        if meta.len() > 200_000 {
            bail!("file too large: {} bytes", meta.len());
        }

        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))
    }

    fn fs_write(&self, user_path: &str, content: &str) -> anyhow::Result<String> {
        let path = self.resolve_workspace_path_for_write(user_path)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create dirs {}", parent.display()))?;
        }

        std::fs::write(&path, content).with_context(|| format!("write {}", path.display()))?;
        Ok("ok".to_string())
    }

    async fn shell(&self, command: &str, timeout_ms: Option<u64>) -> anyhow::Result<String> {
        if command.trim().is_empty() {
            bail!("command is empty");
        }

        // Basic guardrail: avoid obvious foot-guns.
        if command.contains("rm -rf /") || command.contains("sudo ") {
            bail!("command denied by policy");
        }

        let timeout = Duration::from_millis(timeout_ms.unwrap_or(60_000));

        let mut cmd = tokio::process::Command::new("bash");
        cmd.arg("-c")
            .arg(command)
            .current_dir(&self.workspace_root)
            .env_clear()
            .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin");

        let output = tokio::time::timeout(timeout, cmd.output())
            .await
            .context("shell timed out")?
            .context("spawn shell")?;

        let mut combined = String::new();
        combined.push_str(&String::from_utf8_lossy(&output.stdout));
        combined.push_str(&String::from_utf8_lossy(&output.stderr));

        if !output.status.success() {
            bail!("shell failed: {}", combined.trim());
        }

        Ok(combined)
    }

    fn resolve_workspace_path(&self, user_path: &str) -> anyhow::Result<PathBuf> {
        let rel = validate_relative_path(user_path)?;
        let candidate = self.workspace_root.join(&rel);
        self.ensure_no_symlink_escape(&rel)?;
        Ok(candidate)
    }

    fn resolve_workspace_path_for_write(&self, user_path: &str) -> anyhow::Result<PathBuf> {
        let rel = validate_relative_path(user_path)?;
        // For writes, forbid writing to an existing symlink and forbid any symlink components.
        self.ensure_no_symlink_escape(&rel)?;
        let candidate = self.workspace_root.join(&rel);
        if candidate.exists() {
            let ft = std::fs::symlink_metadata(&candidate)?.file_type();
            if ft.is_symlink() {
                bail!("path is a symlink");
            }
        }
        Ok(candidate)
    }

    fn ensure_no_symlink_escape(&self, rel: &Path) -> anyhow::Result<()> {
        let mut cur = self.workspace_root.clone();
        for comp in rel.components() {
            if let Component::Normal(seg) = comp {
                cur.push(seg);
                if cur.exists() {
                    let ft = std::fs::symlink_metadata(&cur)
                        .with_context(|| format!("stat {}", cur.display()))?
                        .file_type();
                    if ft.is_symlink() {
                        bail!("symlinks are not allowed in workspace paths");
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, serde::Deserialize)]
struct FsReadArgs {
    path: String,
}

#[derive(Debug, serde::Deserialize)]
struct FsWriteArgs {
    path: String,
    content: String,
}

#[derive(Debug, serde::Deserialize)]
struct ShellArgs {
    command: String,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

fn validate_relative_path(user_path: &str) -> anyhow::Result<PathBuf> {
    if user_path.trim().is_empty() {
        bail!("path is empty");
    }

    let p = Path::new(user_path);
    if p.is_absolute() {
        bail!("absolute paths are not allowed");
    }

    let mut out = PathBuf::new();
    for comp in p.components() {
        match comp {
            Component::CurDir => {}
            Component::Normal(seg) => out.push(seg),
            Component::ParentDir => bail!("parent traversal is not allowed"),
            Component::RootDir | Component::Prefix(_) => bail!("invalid path"),
        }
    }

    if out.as_os_str().is_empty() {
        bail!("invalid path");
    }
    Ok(out)
}

fn fs_read_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "fs_read".to_string(),
            description: "Read a UTF-8 text file from the workspace.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path inside the workspace." }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
    }
}

fn fs_write_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "fs_write".to_string(),
            description: "Write a UTF-8 text file to the workspace (creates parent dirs).".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path inside the workspace." },
                    "content": { "type": "string", "description": "Full file contents to write." }
                },
                "required": ["path", "content"],
                "additionalProperties": false
            }),
        },
    }
}

fn shell_def() -> ToolDefinition {
    ToolDefinition {
        kind: "function".to_string(),
        function: ToolFunctionDefinition {
            name: "shell".to_string(),
            description: "Run a shell command inside the workspace.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Command to run." },
                    "timeout_ms": { "type": "integer", "description": "Timeout in milliseconds (default 60000).", "minimum": 1 }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_relative_path_rejects_parent_and_absolute() {
        assert!(validate_relative_path("../a").is_err());
        assert!(validate_relative_path("/etc/passwd").is_err());
        assert!(validate_relative_path("").is_err());
        assert!(validate_relative_path(".").is_err());
        assert!(validate_relative_path("./../a").is_err());
    }
}
