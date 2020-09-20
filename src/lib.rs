mod flushing_writer;
mod options;

use anyhow::{Context, Result};
use cargo_metadata::{diagnostic::DiagnosticLevel, CompilerMessage, Message};
use either::Either;
use flushing_writer::FlushingWriter;
use itertools::Itertools;
use options::Options;
use std::{
    env,
    io::{self, BufRead, BufReader, Cursor},
    path::PathBuf,
    process::{Command, Stdio},
};

const CARGO_EXECUTABLE: &str = "cargo";
const CARGO_ENV_VAR: &str = "CARGO";
const NO_EXIT_CODE: i32 = 127;
const BUILD_FINISHED_MESSAGE: &str = r#""build-finished""#;
const ADDITIONAL_ENVIRONMENT_VARIABLES: &str =
    "Additional environment variables:\n    CARGO_LIMIT         Limit compiler messages number (0 \
     means no limit, which is default)\n    CARGO_ASC           Show compiler messages in \
     ascending order (false is default)\n    CARGO_FORCE_WARN    Show warnings even if errors \
     still exist (false is default)";

#[derive(Default)]
struct ParsedMessages {
    internal_compiler_errors: Vec<CompilerMessage>,
    errors: Vec<CompilerMessage>,
    non_errors: Vec<CompilerMessage>,
}

pub fn run_cargo_filtered(cargo_command: &str) -> Result<i32> {
    let parsed_args = Options::from_args_and_vars(cargo_command)?;

    let cargo_path = env::var(CARGO_ENV_VAR)
        .map(PathBuf::from)
        .ok()
        .unwrap_or_else(|| PathBuf::from(CARGO_EXECUTABLE));

    let mut command = Command::new(cargo_path)
        .args(parsed_args.cargo_args.clone())
        .stdout(Stdio::piped())
        .spawn()?;

    let mut reader = BufReader::new(command.stdout.take().context("cannot read stdout")?);

    let help = parsed_args.help;

    if !help {
        let raw_messages = read_raw_messages(&mut reader)?;
        let parsed_messages = ParsedMessages::parse(raw_messages)?;
        let processed_messages = process_messages(parsed_messages, &parsed_args);
        if parsed_args.json_message_format {
            for message in processed_messages {
                println!("{}", serde_json::to_string(&message)?);
            }
        } else {
            for message in processed_messages.filter_map(|message| match message {
                Message::CompilerMessage(compiler_message) => compiler_message.message.rendered,
                _ => None,
            }) {
                print!("{}", message);
            }
        }
    }

    io::copy(&mut reader, &mut FlushingWriter::new(io::stdout()))?;

    if help {
        println!("\n{}", ADDITIONAL_ENVIRONMENT_VARIABLES);
    }

    let exit_code = command.wait()?.code().unwrap_or(NO_EXIT_CODE);
    Ok(exit_code)
}

impl ParsedMessages {
    fn parse(raw_messages: Vec<u8>) -> Result<Self> {
        let mut result = ParsedMessages::default();

        for message in Message::parse_stream(Cursor::new(raw_messages)) {
            if let Message::CompilerMessage(compiler_message) = message? {
                match compiler_message.message.level {
                    DiagnosticLevel::Ice => result.internal_compiler_errors.push(compiler_message),
                    DiagnosticLevel::Error => result.errors.push(compiler_message),
                    _ => result.non_errors.push(compiler_message),
                }
            }
        }

        Ok(result)
    }
}

fn process_messages(
    parsed_messages: ParsedMessages,
    parsed_args: &Options,
) -> impl Iterator<Item = Message> {
    let messages = if parsed_args.show_warnings_if_errors_exist {
        Either::Left(
            parsed_messages
                .internal_compiler_errors
                .into_iter()
                .chain(parsed_messages.errors.into_iter())
                .chain(parsed_messages.non_errors.into_iter()),
        )
    } else {
        let has_any_errors = !parsed_messages.internal_compiler_errors.is_empty()
            || !parsed_messages.errors.is_empty();
        let messages = if has_any_errors {
            Either::Left(
                parsed_messages
                    .internal_compiler_errors
                    .into_iter()
                    .chain(parsed_messages.errors.into_iter()),
            )
        } else {
            Either::Right(parsed_messages.non_errors.into_iter())
        };
        Either::Right(messages)
    };

    let messages = messages.unique();

    let limit_messages = parsed_args.limit_messages;
    let no_limit = limit_messages == 0;
    let messages = if no_limit {
        Either::Left(messages)
    } else {
        Either::Right(messages.take(limit_messages))
    };

    let messages = messages.collect::<Vec<_>>().into_iter();
    let messages = if parsed_args.ascending_messages_order {
        Either::Left(messages)
    } else {
        Either::Right(messages.rev())
    };

    messages.map(Message::CompilerMessage)
}

fn read_raw_messages<R: io::Read>(reader: &mut BufReader<R>) -> Result<Vec<u8>> {
    let mut line = String::new();
    let mut raw_messages = Vec::new();

    loop {
        let len = reader.read_line(&mut line)?;
        raw_messages.extend(line.as_bytes());
        if len == 0 || line.contains(BUILD_FINISHED_MESSAGE) {
            break;
        }
        line.clear();
    }

    Ok(raw_messages)
}
