mod flushing_writer;
mod options;

use anyhow::{Context, Result};
use cargo_metadata::{diagnostic::DiagnosticLevel, Message};
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

#[derive(Default)]
struct ParsedMessages {
    internal_compiler_errors: Vec<String>,
    errors: Vec<String>,
    non_errors: Vec<String>,
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
        parse_and_process_messages(raw_messages, parsed_args)?;
    }

    io::copy(&mut reader, &mut FlushingWriter::new(io::stdout()))?;

    let exit_code = command.wait()?.code().unwrap_or(NO_EXIT_CODE);
    Ok(exit_code)
}

fn parse_and_process_messages(raw_messages: Vec<u8>, parsed_args: Options) -> Result<()> {
    let mut parsed_messages = ParsedMessages::default();

    for message in cargo_metadata::Message::parse_stream(Cursor::new(raw_messages)) {
        if let Message::CompilerMessage(compiler_message) = message? {
            if let Some(rendered) = compiler_message.message.rendered {
                match compiler_message.message.level {
                    DiagnosticLevel::Ice => parsed_messages.internal_compiler_errors.push(rendered),
                    DiagnosticLevel::Error => parsed_messages.errors.push(rendered),
                    _ => parsed_messages.non_errors.push(rendered),
                }
            }
        }
    }

    for message in process_messages(parsed_messages, parsed_args) {
        print!("{}", message);
    }

    Ok(())
}

fn process_messages(
    parsed_messages: ParsedMessages,
    parsed_args: Options,
) -> impl Iterator<Item = String> {
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
    if parsed_args.ascending_messages_order {
        Either::Left(messages)
    } else {
        Either::Right(messages.rev())
    }
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
