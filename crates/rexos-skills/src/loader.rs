use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::manifest::{SkillManifest, parse_manifest};

const SKILL_MANIFEST_FILE: &str = "skill.toml";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillSource {
    Home,
    WorkspaceLegacy,
    Workspace,
}

#[derive(Debug, Clone)]
pub struct DiscoveredSkill {
    pub name: String,
    pub root_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub source: SkillSource,
    pub manifest: SkillManifest,
}

pub fn discover_skills(
    workspace_root: &Path,
    home_skills_root: &Path,
) -> anyhow::Result<BTreeMap<String, DiscoveredSkill>> {
    let mut discovered = BTreeMap::new();

    // Lower precedence first, later inserts override.
    let roots = [
        (SkillSource::Home, home_skills_root.to_path_buf()),
        (
            SkillSource::WorkspaceLegacy,
            workspace_root.join(".rexos/skills"),
        ),
        (SkillSource::Workspace, workspace_root.join(".loopforge/skills")),
    ];

    for (source, root) in roots {
        discover_under_root(&root, source, &mut discovered)?;
    }

    Ok(discovered)
}

fn discover_under_root(
    root: &Path,
    source: SkillSource,
    out: &mut BTreeMap<String, DiscoveredSkill>,
) -> anyhow::Result<()> {
    if !root.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let skill_root = entry.path();
        let manifest_path = skill_root.join(SKILL_MANIFEST_FILE);
        if !manifest_path.is_file() {
            continue;
        }

        let raw = match std::fs::read_to_string(&manifest_path) {
            Ok(raw) => raw,
            Err(_) => continue,
        };

        let manifest = match parse_manifest(&raw) {
            Ok(manifest) => manifest,
            Err(_) => continue,
        };
        let name = manifest.name.clone();
        out.insert(
            name.clone(),
            DiscoveredSkill {
                name,
                root_dir: skill_root,
                manifest_path,
                source,
                manifest,
            },
        );
    }

    Ok(())
}
