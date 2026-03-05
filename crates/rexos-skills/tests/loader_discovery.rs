use rexos_skills::loader::{SkillSource, discover_skills};

fn write_manifest(dir: &std::path::Path, name: &str, version: &str) {
    std::fs::create_dir_all(dir).unwrap();
    let manifest = format!(
        "name = \"{name}\"\nversion = \"{version}\"\nentry = \"SKILL.md\"\n"
    );
    std::fs::write(dir.join("skill.toml"), manifest).unwrap();
}

#[test]
fn workspace_skills_override_global_skills() {
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("workspace");
    let home_skills = tmp.path().join("home/.codex/skills");

    write_manifest(&home_skills.join("write-plan"), "write-plan", "0.1.0");
    write_manifest(
        &workspace.join(".loopforge/skills/write-plan"),
        "write-plan",
        "0.2.0",
    );

    let resolved = discover_skills(&workspace, &home_skills).unwrap();
    let skill = resolved.get("write-plan").unwrap();

    assert_eq!(skill.source, SkillSource::Workspace);
    assert_eq!(skill.manifest.version.to_string(), "0.2.0");
}

#[test]
fn legacy_workspace_skills_override_home_when_loopforge_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("workspace");
    let home_skills = tmp.path().join("home/.codex/skills");

    write_manifest(&home_skills.join("hello"), "hello", "0.1.0");
    write_manifest(&workspace.join(".rexos/skills/hello"), "hello", "0.3.0");

    let resolved = discover_skills(&workspace, &home_skills).unwrap();
    let skill = resolved.get("hello").unwrap();

    assert_eq!(skill.source, SkillSource::WorkspaceLegacy);
    assert_eq!(skill.manifest.version.to_string(), "0.3.0");
}
