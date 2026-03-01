#[test]
fn harness_session_id_is_persisted_per_workspace() {
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("ws");
    rexos::harness::init_workspace(&workspace).unwrap();

    let s1 = rexos::harness::resolve_session_id(&workspace).unwrap();
    let s2 = rexos::harness::resolve_session_id(&workspace).unwrap();
    assert_eq!(s1, s2);

    let on_disk = std::fs::read_to_string(workspace.join(".rexos/session_id")).unwrap();
    assert_eq!(on_disk.trim(), s1);

    let ignore = std::fs::read_to_string(workspace.join(".gitignore")).unwrap();
    assert!(ignore.lines().any(|l| l.trim() == ".rexos/"));
}

