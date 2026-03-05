use std::path::{Path, PathBuf};

use anyhow::Context;

#[derive(Debug, Clone)]
pub struct RexosPaths {
    pub base_dir: PathBuf,
}

impl RexosPaths {
    pub fn discover() -> anyhow::Result<Self> {
        let home_dir = dirs::home_dir().context("could not resolve home directory")?;
        Ok(Self {
            base_dir: home_dir.join(".rexos"),
        })
    }

    pub fn config_path(&self) -> PathBuf {
        self.base_dir.join("config.toml")
    }

    pub fn db_path(&self) -> PathBuf {
        self.base_dir.join("rexos.db")
    }

    pub fn workspace_skills_dir(workspace_root: &Path) -> PathBuf {
        workspace_root.join(".loopforge/skills")
    }

    pub fn workspace_legacy_skills_dir(workspace_root: &Path) -> PathBuf {
        workspace_root.join(".rexos/skills")
    }

    pub fn codex_home_skills_dir(home_dir: &Path) -> PathBuf {
        home_dir.join(".codex/skills")
    }

    pub fn ensure_dirs(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.base_dir)
            .with_context(|| format!("create base dir: {}", self.base_dir.display()))?;
        Ok(())
    }

    pub fn is_inside_base(&self, candidate: &Path) -> bool {
        candidate.starts_with(&self.base_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_inside_base_checks_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = RexosPaths {
            base_dir: tmp.path().to_path_buf(),
        };

        assert!(paths.is_inside_base(&paths.base_dir.join("a/b/c")));
        assert!(!paths.is_inside_base(Path::new("/tmp/not-rexos")));
    }

    #[test]
    fn skills_paths_follow_expected_layout() {
        let workspace = Path::new("/tmp/workspace");
        let home = Path::new("/tmp/home");

        assert_eq!(
            RexosPaths::workspace_skills_dir(workspace),
            PathBuf::from("/tmp/workspace/.loopforge/skills")
        );
        assert_eq!(
            RexosPaths::workspace_legacy_skills_dir(workspace),
            PathBuf::from("/tmp/workspace/.rexos/skills")
        );
        assert_eq!(
            RexosPaths::codex_home_skills_dir(home),
            PathBuf::from("/tmp/home/.codex/skills")
        );
    }
}
