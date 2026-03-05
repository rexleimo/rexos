use rexos_skills::manifest::parse_manifest;

#[test]
fn rejects_manifest_without_name() {
    let raw = r#"
version = "0.1.0"
entry = "SKILL.md"
"#;

    let err = parse_manifest(raw).unwrap_err();
    assert!(err.to_string().contains("name"));
}

#[test]
fn parses_valid_manifest_and_defaults_optional_fields() {
    let raw = r#"
name = "hello-skill"
version = "0.1.0"
entry = "SKILL.md"
"#;

    let manifest = parse_manifest(raw).unwrap();
    assert_eq!(manifest.name, "hello-skill");
    assert_eq!(manifest.version.to_string(), "0.1.0");
    assert_eq!(manifest.entry, "SKILL.md");
    assert!(manifest.permissions.is_empty());
    assert!(manifest.dependencies.is_empty());
}
