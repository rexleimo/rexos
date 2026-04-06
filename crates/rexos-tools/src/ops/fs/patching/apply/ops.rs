use crate::patch::{PatchApplyResult, PatchOp};
use crate::Toolset;

use super::files;

pub(super) fn apply_patch_op(
    tools: &Toolset,
    op: PatchOp,
    result: &mut PatchApplyResult,
) -> anyhow::Result<()> {
    match op {
        PatchOp::Add { path, content } => {
            files::write_new_file(tools, &path, content)?;
            result.files_added += 1;
        }
        PatchOp::Update { path, hunks } => {
            files::rewrite_file(tools, &path, hunks)?;
            result.files_updated += 1;
        }
        PatchOp::Delete { path } => {
            files::remove_file(tools, &path)?;
            result.files_deleted += 1;
        }
    }
    Ok(())
}
