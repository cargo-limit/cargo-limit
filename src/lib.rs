mod cargo_toml;
mod io;
mod messages;
mod options;
mod process;

use anyhow::{Context, Result};
use cargo_metadata::{diagnostic::DiagnosticSpan, Message, MetadataCommand};
use io::Buffers;
use messages::{process_messages, ParsedMessages, ProcessedMessages};
use options::Options;
use std::{
    env, fmt,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const CARGO_EXECUTABLE: &str = "cargo";
const CARGO_ENV_VAR: &str = "CARGO";
const NO_EXIT_CODE: i32 = 127;

const ADDITIONAL_ENVIRONMENT_VARIABLES: &str =
    include_str!("../additional_environment_variables.txt");

#[doc(hidden)]
pub fn run_cargo_filtered(cargo_command: &str) -> Result<i32> {
    let workspace_root = MetadataCommand::new().exec()?.workspace_root;
    let parsed_args = Options::from_args_and_vars(cargo_command, &workspace_root)?;
    let cargo_path = env::var(CARGO_ENV_VAR)
        .map(PathBuf::from)
        .ok()
        .unwrap_or_else(|| PathBuf::from(CARGO_EXECUTABLE));

    let error_text = failed_to_execute_error_text(&cargo_path);
    let mut child = Command::new(cargo_path)
        .args(parsed_args.cargo_args.clone())
        .stdout(Stdio::piped())
        .spawn()
        .context(error_text)?;

    let cargo_pid = child.id();
    ctrlc::set_handler(move || {
        process::kill(cargo_pid);
    })?;

    let mut buffers = Buffers::new(&mut child)?;

    parse_and_process_messages(&mut buffers, cargo_pid, &parsed_args, workspace_root)?;
    buffers.copy_from_child_stdout_reader_to_stdout_writer()?;

    let exit_code = child.wait()?.code().unwrap_or(NO_EXIT_CODE);
    // TODO: process messages again
    //buffers.copy_from_child_reader_to_stdout_writer()?;

    if parsed_args.help {
        buffers.write_to_stdout(ADDITIONAL_ENVIRONMENT_VARIABLES)?;
    }

    Ok(exit_code)
}

fn parse_and_process_messages(
    buffers: &mut Buffers,
    cargo_pid: u32,
    parsed_args: &Options,
    workspace_root: PathBuf,
) -> Result<()> {
    if parsed_args.help || parsed_args.version {
        return Ok(());
    }

    let parsed_messages =
        ParsedMessages::parse(buffers.child_stdout_reader_mut(), cargo_pid, parsed_args)?;
    let ProcessedMessages {
        messages,
        spans_in_consistent_order,
    } = process_messages(parsed_messages, &parsed_args, &workspace_root)?;
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

    open_in_external_application_for_affected_files(buffers, spans_in_consistent_order, parsed_args)
}

fn open_in_external_application_for_affected_files(
    buffers: &mut Buffers,
    spans_in_consistent_order: Vec<DiagnosticSpan>,
    parsed_args: &Options,
) -> Result<()> {
    let open_in_external_application = &parsed_args.open_in_external_application;
    if !open_in_external_application.is_empty() {
        let mut args = Vec::new();
        for span in spans_in_consistent_order.into_iter() {
            args.push(format!(
                "{}:{}:{}",
                span.file_name, span.line_start, span.column_start
            ));
        }
        if !args.is_empty() {
            let error_text = failed_to_execute_error_text(open_in_external_application);
            let output = Command::new(open_in_external_application)
                .args(args)
                .output()
                .context(error_text)?;
            buffers.write_all_to_stderr(&output.stdout)?;
            buffers.write_all_to_stderr(&output.stderr)?;
        }
    }
    Ok(())
}

fn failed_to_execute_error_text<T: fmt::Debug>(program: T) -> String {
    format!("failed to execute {:?}", program)
}

#[doc(hidden)]
#[macro_export]
macro_rules! run_command {
    ($command:expr) => {
        fn main() -> anyhow::Result<()> {
            std::process::exit(cargo_limit::run_cargo_filtered($command)?);
        }
    };
}
