use anyhow::bail;

use super::super::hunk::parse_hunk;
use crate::patch::PatchOp;

pub(super) fn parse_update_file(
    path: String,
    body: &[&str],
    index: &mut usize,
) -> anyhow::Result<PatchOp> {
    let mut hunks = Vec::new();
    while *index < body.len() && !body[*index].trim().starts_with("***") {
        let current = body[*index].trim();
        if current.starts_with("@@") {
            *index += 1;
            hunks.push(parse_hunk(body, index));
        } else {
            *index += 1;
        }
    }

    if hunks.is_empty() {
        bail!("Update File '{path}' has no hunks");
    }

    Ok(PatchOp::Update { path, hunks })
}
