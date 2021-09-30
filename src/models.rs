use cargo_metadata::diagnostic::{Diagnostic, DiagnosticLevel, DiagnosticSpan};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Deserialize, Serialize)]
pub struct EditorData {
    pub workspace_root: PathBuf,
    pub files: Vec<SourceFile>,
}

#[derive(Deserialize, Serialize)]
pub struct SourceFile {
    relative_path: PathBuf,
    line: usize,
    column: usize,
    message: String,
    level: DiagnosticLevel,
}

impl EditorData {
    pub fn new(workspace_root: &Path, source_files_in_consistent_order: Vec<SourceFile>) -> Self {
        let workspace_root = workspace_root.to_path_buf();
        Self {
            workspace_root,
            files: source_files_in_consistent_order,
        }
    }
}

impl SourceFile {
    pub fn new(span: DiagnosticSpan, diagnostic: &Diagnostic) -> Self {
        Self {
            relative_path: PathBuf::from(span.file_name),
            line: span.line_start,
            column: span.column_start,
            message: diagnostic.message.clone(),
            level: diagnostic.level,
        }
    }
}
