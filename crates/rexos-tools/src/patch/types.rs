#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PatchOp {
    Add { path: String, content: String },
    Update { path: String, hunks: Vec<PatchHunk> },
    Delete { path: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PatchHunk {
    pub(crate) old_lines: Vec<String>,
    pub(crate) new_lines: Vec<String>,
}

#[derive(Debug, Default)]
pub(crate) struct PatchApplyResult {
    pub(crate) files_added: u32,
    pub(crate) files_updated: u32,
    pub(crate) files_deleted: u32,
}

impl PatchApplyResult {
    pub(crate) fn summary(&self) -> String {
        let mut parts = Vec::new();
        if self.files_added > 0 {
            parts.push(format!("{} added", self.files_added));
        }
        if self.files_updated > 0 {
            parts.push(format!("{} updated", self.files_updated));
        }
        if self.files_deleted > 0 {
            parts.push(format!("{} deleted", self.files_deleted));
        }
        if parts.is_empty() {
            "No changes applied".to_string()
        } else {
            parts.join(", ")
        }
    }
}
