use anyhow::Result;
use cargo_metadata::diagnostic::DiagnosticSpan;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// TODO: naming. EditorCall?
#[derive(Deserialize, Serialize)]
pub struct EditorData {
    pub workspace_root: PathBuf,
    pub files: Vec<SourceFile>,
}

// TODO: common struct?
#[derive(Deserialize, Serialize)]
pub struct SourceFile {
    pub relative_path: String, // TODO: PathBuf?
    pub line: usize,
    pub column: usize,
}

impl EditorData {
    pub fn new(workspace_root: &Path, spans_in_consistent_order: Vec<DiagnosticSpan>) -> Self {
        let workspace_root = workspace_root.to_path_buf();
        let files = spans_in_consistent_order
            .into_iter()
            .rev()
            .map(SourceFile::from_diagnostic_span)
            .collect();
        Self {
            workspace_root,
            files,
        }
    }

    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl SourceFile {
    pub fn from_diagnostic_span(span: DiagnosticSpan) -> Self {
        Self {
            relative_path: span.file_name,
            line: span.line_start,
            column: span.column_start,
        }
    }
}
