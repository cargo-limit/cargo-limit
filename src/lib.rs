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
    env, fmt,
    io::{self, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdout, Command, Stdio},
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

    let help = parsed_args.help;
    let version = parsed_args.version;

    if !help && !version {
        let parsed_messages =
            ParsedMessages::parse(&mut buffers.stdout_reader, cargo_pid, &parsed_args)?;
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

        let open_in_external_application = parsed_args.open_in_external_application;
        if !open_in_external_application.is_empty() {
            let mut args = Vec::new();
            for span in spans_in_consistent_order.into_iter() {
                args.push(format!(
                    "{}:{}:{}",
                    span.file_name, span.line_start, span.column_start
                ));
            }
            if !args.is_empty() {
                let error_text = failed_to_execute_error_text(&open_in_external_application);
                let output = Command::new(open_in_external_application)
                    .args(args)
                    .output()
                    .context(error_text)?;
                buffers.stderr_writer.write_all(&output.stdout)?;
                buffers.stderr_writer.write_all(&output.stderr)?;
            }
        }
    }

    buffers.copy_from_child_reader_to_stdout_writer()?;

    if help {
        std::write!(
            &mut buffers.stdout_writer,
            "{}",
            ADDITIONAL_ENVIRONMENT_VARIABLES
        )?;
        // TODO: do it after wait?
    }

    let exit_code = child.wait()?.code().unwrap_or(NO_EXIT_CODE);
    // TODO: process messages again
    //buffers.copy_from_child_reader_to_stdout_writer()?;
    Ok(exit_code)
}

// TODO: move
struct Buffers {
    stdout_reader: BufReader<ChildStdout>,
    stdout_writer: FlushingWriter<io::Stdout>,
    stderr_writer: FlushingWriter<io::Stderr>,
}

impl Buffers {
    pub fn new(child: &mut Child) -> Result<Self> {
        let stdout_reader = BufReader::new(child.stdout.take().context("cannot read stdout")?);
        let stdout_writer = FlushingWriter::new(io::stdout());
        let stderr_writer = FlushingWriter::new(io::stderr());
        Ok(Self {
            stdout_reader,
            stdout_writer,
            stderr_writer,
        })
    }

    pub fn writeln_to_stdout(&mut self, text: String) -> io::Result<()> {
        std::writeln!(&mut self.stdout_writer, "{}", text)
    }

    pub fn write_to_stderr(&mut self, text: String) -> io::Result<()> {
        std::write!(&mut self.stderr_writer, "{}", text)
    }

    pub fn copy_from_child_reader_to_stdout_writer(&mut self) -> io::Result<u64> {
        io::copy(&mut self.stdout_reader, &mut self.stdout_writer)
    }
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
