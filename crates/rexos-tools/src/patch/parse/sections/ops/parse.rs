use crate::patch::PatchOp;

use super::super::body::patch_body;
use super::directive::{parse_patch_directive, PatchDirective};

pub(super) fn parse_patch(input: &str) -> anyhow::Result<Vec<PatchOp>> {
    let lines: Vec<&str> = input.lines().collect();
    let body = patch_body(&lines)?;
    let mut ops = Vec::new();
    let mut index = 0usize;

    while index < body.len() {
        let line = body[index].trim();
        if line.is_empty() {
            index += 1;
            continue;
        }

        index += 1;
        match parse_patch_directive(line)? {
            PatchDirective::Add(path) => {
                ops.push(super::add::parse_add_file(path, body, &mut index)?)
            }
            PatchDirective::Update(path) => {
                ops.push(super::update::parse_update_file(path, body, &mut index)?)
            }
            PatchDirective::Delete(path) => ops.push(PatchOp::Delete { path }),
        }
    }

    Ok(ops)
}
