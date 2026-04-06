use anyhow::bail;

use crate::patch::PatchOp;

pub(super) fn parse_add_file(
    path: String,
    body: &[&str],
    index: &mut usize,
) -> anyhow::Result<PatchOp> {
    let mut content_lines = Vec::new();
    while *index < body.len() && !body[*index].trim().starts_with("***") {
        let raw = body[*index];
        if let Some(stripped) = raw.strip_prefix('+') {
            content_lines.push(stripped.to_string());
        } else if raw.trim().is_empty() {
            content_lines.push(String::new());
        } else {
            bail!("expected '+' prefix in Add File content, got: {}", raw);
        }
        *index += 1;
    }

    Ok(PatchOp::Add {
        path,
        content: content_lines.join("\n"),
    })
}
