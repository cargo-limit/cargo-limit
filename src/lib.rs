mod cargo_toml;
mod flushing_writer;
mod messages;
mod options;
mod process;

use anyhow::{Context, Result};
use cargo_metadata::{Message, MetadataCommand};
use flushing_writer::FlushingWriter;
use messages::{process_messages, ParsedMessages, ProcessedMessages};
use options::Options;
use std::{
    env,
    io::{self, BufReader, Write},
    path::PathBuf,
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
    let mut command = Command::new(cargo_path)
        .args(parsed_args.cargo_args.clone())
        .stdout(Stdio::piped())
        .spawn()?;

    let cargo_pid = command.id();
    ctrlc::set_handler(move || {
        process::kill(cargo_pid);
    })?;

    let mut stdout_reader = BufReader::new(command.stdout.take().context("cannot read stdout")?);
    let mut stdout_writer = FlushingWriter::new(io::stdout());

    let help = parsed_args.help;

    if !help {
        let parsed_messages = ParsedMessages::parse(&mut stdout_reader, cargo_pid, &parsed_args)?;
        let ProcessedMessages {
            messages,
            spans_in_consistent_order,
        } = process_messages(parsed_messages, &parsed_args, &workspace_root)?;
        let processed_messages = messages.into_iter();

        let open_in_external_application = parsed_args.open_in_external_application;
        if !open_in_external_application.is_empty() {
            let mut args = Vec::new();
            for span in spans_in_consistent_order.into_iter() {
                args.push(format!(
                    "{}:{}:{}",
                    span.file_name, span.line_start, span.column_start
                ));
            }
            let output = Command::new(open_in_external_application)
                .args(args)
                .output()?;
            io::stderr().write_all(&output.stdout)?;
            io::stderr().write_all(&output.stderr)?;
        }

        if parsed_args.json_message_format {
            for message in processed_messages {
                println!("{}", serde_json::to_string(&message)?);
            }
        } else {
            for message in processed_messages.filter_map(|message| match message {
                Message::CompilerMessage(compiler_message) => compiler_message.message.rendered,
                _ => None,
            }) {
                eprint!("{}", message);
            }
        }
    }

    io::copy(&mut stdout_reader, &mut stdout_writer)?;

    if help {
        print!("{}", ADDITIONAL_ENVIRONMENT_VARIABLES);
    }

    let exit_code = command.wait()?.code().unwrap_or(NO_EXIT_CODE);
    Ok(exit_code)
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
