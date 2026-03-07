use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context};

use crate::patch::{apply_hunks_to_text, parse_patch, PatchApplyResult, PatchOp};
use crate::Toolset;

impl Toolset {
    pub(crate) fn fs_read(&self, user_path: &str) -> anyhow::Result<String> {
        let path = self.resolve_workspace_path(user_path)?;

        let meta = std::fs::metadata(&path).with_context(|| format!("stat {}", path.display()))?;
        if meta.len() > 200_000 {
            bail!("file too large: {} bytes", meta.len());
        }

        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))
    }

    pub(crate) fn fs_write(&self, user_path: &str, content: &str) -> anyhow::Result<String> {
        let path = self.resolve_workspace_path_for_write(user_path)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create dirs {}", parent.display()))?;
        }

        std::fs::write(&path, content).with_context(|| format!("write {}", path.display()))?;
        Ok("ok".to_string())
    }

    pub(crate) fn file_list(&self, user_path: &str) -> anyhow::Result<String> {
        let resolved = if user_path.trim() == "." {
            self.workspace_root.clone()
        } else {
            self.resolve_workspace_path(user_path)?
        };

        let mut out = Vec::new();
        for entry in std::fs::read_dir(&resolved)
            .with_context(|| format!("list dir {}", resolved.display()))?
        {
            let entry = entry.context("read dir entry")?;
            let name = entry.file_name().to_string_lossy().to_string();
            let suffix = match entry.file_type() {
                Ok(ft) if ft.is_dir() => "/",
                _ => "",
            };
            out.push(format!("{name}{suffix}"));
        }
        out.sort();
        Ok(out.join("\n"))
    }

    pub(crate) fn apply_patch(&self, patch: &str) -> anyhow::Result<String> {
        let ops = parse_patch(patch).context("parse patch")?;
        let mut result = PatchApplyResult::default();

        for op in ops {
            match op {
                PatchOp::AddFile { path, content } => {
                    let dest = self.resolve_workspace_path_for_write(&path)?;
                    if let Some(parent) = dest.parent() {
                        std::fs::create_dir_all(parent)
                            .with_context(|| format!("create dirs {}", parent.display()))?;
                    }
                    std::fs::write(&dest, content)
                        .with_context(|| format!("write {}", dest.display()))?;
                    result.files_added += 1;
                }
                PatchOp::UpdateFile { path, hunks } => {
                    let dest = self.resolve_workspace_path_for_write(&path)?;
                    let before = std::fs::read_to_string(&dest)
                        .with_context(|| format!("read {}", dest.display()))?;
                    let after = apply_hunks_to_text(&before, &hunks).context("apply hunks")?;
                    std::fs::write(&dest, after)
                        .with_context(|| format!("write {}", dest.display()))?;
                    result.files_updated += 1;
                }
                PatchOp::DeleteFile { path } => {
                    let dest = self.resolve_workspace_path(&path)?;
                    std::fs::remove_file(&dest)
                        .with_context(|| format!("delete {}", dest.display()))?;
                    result.files_deleted += 1;
                }
            }
        }

        Ok(result.summary())
    }

    pub(crate) fn resolve_workspace_path(&self, user_path: &str) -> anyhow::Result<PathBuf> {
        let rel = validate_relative_path(user_path)?;
        let candidate = self.workspace_root.join(&rel);
        self.ensure_no_symlink_escape(&rel)?;
        Ok(candidate)
    }

    pub(crate) fn resolve_workspace_path_for_write(
        &self,
        user_path: &str,
    ) -> anyhow::Result<PathBuf> {
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

    pub(crate) fn ensure_no_symlink_escape(&self, rel: &Path) -> anyhow::Result<()> {
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

pub(crate) fn validate_relative_path(user_path: &str) -> anyhow::Result<PathBuf> {
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
