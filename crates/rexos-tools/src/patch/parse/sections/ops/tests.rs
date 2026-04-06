use crate::patch::PatchOp;

#[test]
fn parse_patch_rejects_unknown_directive() {
    let patch = r#"*** Begin Patch
*** Strange File: nope.txt
*** End Patch"#;
    let err = super::parse_patch(patch).unwrap_err();
    assert!(err.to_string().contains("unknown patch directive"), "{err}");
}

#[test]
fn parse_patch_skips_blank_lines_between_directives() {
    let patch = r#"*** Begin Patch

*** Delete File: old.txt

*** End Patch"#;
    let ops = super::parse_patch(patch).unwrap();
    assert!(matches!(&ops[0], PatchOp::Delete { path } if path == "old.txt"));
}
