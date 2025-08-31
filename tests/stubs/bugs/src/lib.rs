//! **Documentation is [here](https://github.com/alopatindev/cargo-limit#readme).**

#[doc(hidden)]
pub mod models;

mod cargo_toml;
mod io;
mod messages;
mod options;
mod process;

#[doc(hidden)]
pub use process::NO_EXIT_CODE;

use crate::models::{EditorData, Location};
use anyhow::{Context, Result};
use cargo_metadata::{Message, MetadataCommand};
use io::Buffers;
use messages::{transform_and_process_messages, Messages};
use options::Options;
use process::{failed_to_execute_error_text, CargoProcess};
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

const ADDITIONAL_ENVIRONMENT_VARIABLES: &str = "";
