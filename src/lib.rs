//! **Documentation is [here](https://github.com/alopatindev/cargo-limit#readme).**

#[doc(hidden)]
pub mod models;

mod cargo_toml;
mod io;
mod messages;
mod options;
mod process;

use anyhow::{Context, Result};
use cargo_metadata::{diagnostic::DiagnosticSpan, Message, MetadataCommand};
use io::Buffers;
use messages::{process_messages, ParsedMessages, ProcessedMessages};
use models::EditorData;
use options::Options;
use std::{
    env, fmt,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub(crate) const CARGO_EXECUTABLE: &str = "cargo";
const CARGO_ENV_VAR: &str = "CARGO";

#[doc(hidden)]
pub const NO_EXIT_CODE: i32 = 127;

const ADDITIONAL_ENVIRONMENT_VARIABLES: &str =
    include_str!("../additional_environment_variables.txt");

#[doc(hidden)]
pub fn run_cargo_filtered(current_exe: String) -> Result<i32> {
    let workspace_root = MetadataCommand::new().exec()?.workspace_root;
    let parsed_args = Options::from_os_env(current_exe, &workspace_root)?;
    let cargo_path = env::var(CARGO_ENV_VAR)
        .map(PathBuf::from)
        .ok()
        .unwrap_or_else(|| PathBuf::from(CARGO_EXECUTABLE));

    let error_text = failed_to_execute_error_text(&cargo_path);
    let mut child = Command::new(cargo_path)
        .args(parsed_args.all_args())
        .stdout(Stdio::piped()) // TODO: stderr?
        .spawn()
        .context(error_text)?;

    let cargo_pid = child.id();
    ctrlc::set_handler(move || {
        process::kill(cargo_pid);
    })?;

    let mut buffers = Buffers::new(&mut child)?;
    parse_and_process_messages(&mut buffers, cargo_pid, &parsed_args, &workspace_root)?;
    let exit_code = child.wait()?.code().unwrap_or(NO_EXIT_CODE);
    parse_and_process_messages(&mut buffers, cargo_pid, &parsed_args, &workspace_root)?;

    if parsed_args.help {
        buffers.write_to_stdout(ADDITIONAL_ENVIRONMENT_VARIABLES)?;
    }

    Ok(exit_code)
}

fn parse_and_process_messages(
    buffers: &mut Buffers,
    cargo_pid: u32,
    parsed_args: &Options,
    workspace_root: &Path,
) -> Result<()> {
    if !parsed_args.help && !parsed_args.version {
        let parsed_messages =
            ParsedMessages::parse(buffers.child_stdout_reader_mut(), cargo_pid, parsed_args)?;
        let ProcessedMessages {
            messages,
            spans_in_consistent_order,
        } = process_messages(parsed_messages, &parsed_args, workspace_root)?;
        let processed_messages = messages.into_iter();

        if parsed_args.json_message_format {
            for message in processed_messages {
                buffers.writeln_to_stdout(serde_json::to_string(&message)?)?;
            }
        } else {
            for message in processed_messages.filter_map(|message| match message {
                Message::CompilerMessage(compiler_message) => compiler_message.message.rendered,
                _ => None,
            }) {
                buffers.write_to_stderr(message)?;
            }
        }

        open_in_external_app_for_affected_files(
            buffers,
            spans_in_consistent_order,
            parsed_args,
            workspace_root,
        )?;
    }

    buffers.copy_from_child_stdout_reader_to_stdout_writer()?;
    Ok(())
}

fn open_in_external_app_for_affected_files(
    buffers: &mut Buffers,
    spans_in_consistent_order: Vec<DiagnosticSpan>,
    parsed_args: &Options,
    workspace_root: &Path,
) -> Result<()> {
    let app = &parsed_args.open_in_external_app;
    if !app.is_empty() {
        // TODO: naming?
        let editor_data = EditorData::new(workspace_root, spans_in_consistent_order);
        if !editor_data.files.is_empty() {
            let mut child = Command::new(app)
                .stdin(Stdio::piped())
                //.stdout(Stdio::piped()) // TODO
                //.stderr(Stdio::piped())
                .spawn()?;
            child
                .stdin
                .take()
                .context("no stdin")?
                .write_all(editor_data.to_json()?.as_bytes())?;

            let error_text = failed_to_execute_error_text(app);
            let output = child.wait_with_output().context(error_text)?;

            buffers.write_all_to_stderr(&output.stdout)?;
            buffers.write_all_to_stderr(&output.stderr)?;
        }
    }
    Ok(())
}

fn failed_to_execute_error_text<T: fmt::Debug>(app: T) -> String {
    format!("failed to execute {:?}", app)
}

#[doc(hidden)]
pub fn run_subcommand() -> anyhow::Result<()> {
    let current_exe = std::env::current_exe()?
        .file_stem()
        .ok_or_else(|| anyhow::format_err!("invalid executable"))?
        .to_string_lossy()
        .to_string();
    std::process::exit(run_cargo_filtered(current_exe)?);
}
