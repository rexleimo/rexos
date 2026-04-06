use super::*;

#[test]
fn parse_patch_handles_add_update_and_delete_ops() {
    let patch = r#"*** Begin Patch
*** Add File: greet.txt
+hi
*** Update File: greet.txt
@@
-hi
+hello
*** Delete File: old.txt
*** End Patch"#;

    let ops = parse_patch(patch).unwrap();
    assert_eq!(ops.len(), 3);
    assert!(
        matches!(&ops[0], PatchOp::Add { path, content } if path == "greet.txt" && content == "hi")
    );
    assert!(
        matches!(&ops[1], PatchOp::Update { path, hunks } if path == "greet.txt" && hunks.len() == 1)
    );
    assert!(matches!(&ops[2], PatchOp::Delete { path } if path == "old.txt"));
}

#[test]
fn apply_hunks_to_text_replaces_matching_block() {
    let before = "alpha
beta
gamma
";
    let hunks = vec![PatchHunk {
        old_lines: vec!["beta".to_string()],
        new_lines: vec!["delta".to_string()],
    }];

    let after = apply_hunks_to_text(before, &hunks).unwrap();
    assert_eq!(
        after,
        "alpha
delta
gamma
"
    );
}
