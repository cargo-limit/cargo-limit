use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct EditorData {
    pub locations: Vec<Location>,
}

#[derive(Deserialize)]
pub struct Location {
    pub path: PathBuf,
}
