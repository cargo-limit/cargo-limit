//! **Documentation is [here](https://github.com/alopatindev/cargo-limit#readme).**

#[doc(hidden)]
pub mod models;

mod messages;
mod options;

use crate::models::{EditorData, Location};
use anyhow::{Context, Result};
use cargo_metadata::{Message, MetadataCommand};
use messages::{process_parsed_messages, Messages};
use options::Options;
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

#[doc(hidden)]
pub fn run_cargo_filtered(current_exe: String) -> Result<i32> {
    let workspace_root = MetadataCommand::new().no_deps().exec()?.workspace_root;
    let workspace_root = workspace_root.as_ref();
    let options = Options::from_os_env(current_exe, workspace_root)?;
    Ok(0)
}
