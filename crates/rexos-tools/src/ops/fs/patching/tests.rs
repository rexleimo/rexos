use crate::patch::{PatchApplyResult, PatchHunk, PatchOp};
use crate::Toolset;

use super::apply::apply_patch_op;

#[test]
fn apply_patch_op_add_file_creates_parent_dirs_and_counts_added_file() {
    let tmp = tempfile::tempdir().unwrap();
    let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();
    let mut result = PatchApplyResult::default();

    apply_patch_op(
        &tools,
        PatchOp::Add {
            path: "nested/greet.txt".to_string(),
            content: "hi".to_string(),
        },
        &mut result,
    )
    .unwrap();

    assert_eq!(
        std::fs::read_to_string(tmp.path().join("nested/greet.txt")).unwrap(),
        "hi"
    );
    assert_eq!(result.files_added, 1);
}

#[test]
fn apply_patch_op_update_file_rewrites_existing_content_and_counts_updated_file() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("greet.txt");
    std::fs::write(&file, "hi\n").unwrap();
    let tools = Toolset::new(tmp.path().to_path_buf()).unwrap();
    let mut result = PatchApplyResult::default();

    apply_patch_op(
        &tools,
        PatchOp::Update {
            path: "greet.txt".to_string(),
            hunks: vec![PatchHunk {
                old_lines: vec!["hi".to_string()],
                new_lines: vec!["hello".to_string()],
            }],
        },
        &mut result,
    )
    .unwrap();

    assert_eq!(std::fs::read_to_string(file).unwrap(), "hello\n");
    assert_eq!(result.files_updated, 1);
}
