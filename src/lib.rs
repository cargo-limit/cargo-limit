//! **Documentation is [here](https://github.com/cargo-limit/cargo-limit#readme).**

#[doc(hidden)]
pub mod env_vars;
#[doc(hidden)]
pub mod models;
#[doc(hidden)]
pub mod process;

mod cargo_toml;
mod io;
mod messages;
mod options;

#[doc(hidden)]
pub use process::NO_EXIT_CODE;

use crate::models::{EditorData, Location};
use anyhow::{Context, Result};
use cargo_metadata::{Message, MetadataCommand};
use io::Buffers;
use messages::{Messages, transform_and_process_messages};
use options::Options;
use process::{CargoProcess, failed_to_execute_error_text};
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

const ADDITIONAL_ENVIRONMENT_VARIABLES: &str =
    include_str!("../additional_environment_variables.txt");

#[doc(hidden)]
pub fn run_cargo_filtered(current_exe: String) -> Result<i32> {
    let workspace_root = MetadataCommand::new()
        .no_deps()
        .exec()
        .ok()
        .map(|m| m.workspace_root);
    let workspace_root = workspace_root.as_ref().map(|w| w.as_ref());
    let options = Options::from_os_env(current_exe, workspace_root)?;

    let mut cargo_process = CargoProcess::run(&options)?;
    let mut buffers = cargo_process.buffers()?;

    let process_messages = |buffers: &mut Buffers,
                            messages: Vec<Message>,
                            locations_in_consistent_order: Vec<Location>,
                            workspace_root: &Path|
     -> Result<()> {
        let messages = messages.into_iter();
        if options.json_message_format {
            for message in messages {
                buffers.writeln_to_stdout(&serde_json::to_string(&message)?)?;
            }
        } else {
            for message in messages.filter_map(|message| match message {
                Message::CompilerMessage(compiler_message) => compiler_message.message.rendered,
                _ => None,
            }) {
                buffers.write_to_stderr(message)?;
            }
        }
        open_affected_files_in_external_app(
            buffers,
            locations_in_consistent_order,
            &options,
            workspace_root,
        )
    };

    let mut parsed_messages =
        Messages::parse_with_timeout_on_error(&mut buffers, Some(&cargo_process), &options)?;

    let exit_code = if parsed_messages.child_killed {
        buffers.writeln_to_stdout("")?;
        let exit_code = cargo_process.wait()?;
        parsed_messages.merge(Messages::parse_with_timeout_on_error(
            &mut buffers,
            None,
            &options,
        )?);
        transform_and_process_messages(
            &mut buffers,
            parsed_messages,
            &options,
            workspace_root,
            process_messages,
        )?;
        buffers.copy_from_child_stdout_reader_to_stdout_writer()?;

        exit_code
    } else {
        transform_and_process_messages(
            &mut buffers,
            parsed_messages,
            &options,
            workspace_root,
            process_messages,
        )?;
        buffers.copy_from_child_stdout_reader_to_stdout_writer()?;
        cargo_process.wait()?
    };

    if options.help {
        buffers.write_to_stdout(ADDITIONAL_ENVIRONMENT_VARIABLES)?;
    }

    Ok(exit_code)
}

fn open_affected_files_in_external_app(
    buffers: &mut Buffers,
    locations_in_consistent_order: Vec<Location>,
    options: &Options,
    workspace_root: &Path,
) -> Result<()> {
    let app = &options.open_in_external_app;
    if !app.is_empty() {
        let editor_data = EditorData::new(workspace_root, locations_in_consistent_order);
        let mut child = Command::new(app).stdin(Stdio::piped()).spawn()?;
        child
            .stdin
            .take()
            .context("no stdin")?
            .write_all(serde_json::to_string(&editor_data)?.as_bytes())?;

        let error_text = failed_to_execute_error_text(app);
        let output = child.wait_with_output().context(error_text)?;

        buffers.write_all_to_stderr(&output.stdout)?;
        buffers.write_all_to_stderr(&output.stderr)?;
    }
    Ok(())
}

#[doc(hidden)]
#[macro_export]
macro_rules! run_subcommand {
    () => {
        #[doc(hidden)]
        fn main() -> anyhow::Result<()> {
            use anyhow::Context;
            let current_exe = std::env::current_exe()?
                .file_stem()
                .context("invalid executable")?
                .to_string_lossy()
                .to_string();
            std::process::exit(cargo_limit::run_cargo_filtered(current_exe)?);
        }
    };
}
