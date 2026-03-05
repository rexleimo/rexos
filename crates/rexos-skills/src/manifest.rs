use anyhow::bail;
use serde::Deserialize;

pub fn parse_manifest(raw: &str) -> anyhow::Result<SkillManifest> {
    let parsed: SkillManifestRaw = toml::from_str(raw)?;

    let name = parsed.name.trim().to_string();
    if name.is_empty() {
        bail!("name cannot be empty");
    }

    let entry = parsed.entry.trim().to_string();
    if entry.is_empty() {
        bail!("entry cannot be empty");
    }

    let permissions = parsed
        .permissions
        .into_iter()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>();

    let mut dependencies = Vec::with_capacity(parsed.dependencies.len());
    for dep in parsed.dependencies {
        let dep_name = dep.name.trim().to_string();
        if dep_name.is_empty() {
            bail!("dependency.name cannot be empty");
        }
        let version_req = dep
            .version_req
            .or(dep.version)
            .unwrap_or(semver::VersionReq::STAR);
        dependencies.push(SkillDependency {
            name: dep_name,
            version_req,
        });
    }

    Ok(SkillManifest {
        name,
        version: parsed.version,
        entry,
        permissions,
        dependencies,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillManifest {
    pub name: String,
    pub version: semver::Version,
    pub entry: String,
    pub permissions: Vec<String>,
    pub dependencies: Vec<SkillDependency>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillDependency {
    pub name: String,
    pub version_req: semver::VersionReq,
}

#[derive(Debug, Clone, Deserialize)]
struct SkillManifestRaw {
    name: String,
    version: semver::Version,
    entry: String,
    #[serde(default)]
    permissions: Vec<String>,
    #[serde(default)]
    dependencies: Vec<SkillDependencyRaw>,
}

#[derive(Debug, Clone, Deserialize)]
struct SkillDependencyRaw {
    name: String,
    #[serde(default)]
    version_req: Option<semver::VersionReq>,
    #[serde(default)]
    version: Option<semver::VersionReq>,
}
