use anyhow::Result;
use cargo_metadata::diagnostic::DiagnosticSpan;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// TODO: naming. EditorCall?
#[derive(Deserialize, Serialize)]
pub struct EditorData {
    pub workspace_root: PathBuf,
    pub files: Vec<SourceFile>, // TODO: naming
}

// TODO: naming
// TODO: common struct?
// TODO: pub fields?
#[derive(Deserialize, Serialize)]
pub struct SourceFile {
    pub relative_path: String, // TODO: PathBuf?
    pub line: usize,
    pub column: usize,
    pub message: String,
    //pub level // TODO
}

impl EditorData {
    pub fn new(workspace_root: &Path, spans_in_consistent_order: Vec<SourceFile>) -> Self {
        let workspace_root = workspace_root.to_path_buf();
        Self {
            workspace_root,
            files: spans_in_consistent_order, // TODO: naming
        }
    }

    // TODO: from ...

    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self)?)
    }
}
