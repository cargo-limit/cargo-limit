mod flushing_writer;
mod parsed_args;

use anyhow::{Context, Result};
use cargo_metadata::{diagnostic::DiagnosticLevel, Message};
use either::Either;
use flushing_writer::FlushingWriter;
use parsed_args::ParsedArgs;
use std::{
    env,
    io::{self, BufRead, BufReader, Cursor},
    iter,
    path::PathBuf,
    process::{Command, Stdio},
};

const MESSAGE_FORMAT: &str = "--message-format=json-diagnostic-rendered-ansi";
const CARGO_EXECUTABLE: &str = "cargo";
const CARGO_ENV_VAR: &str = "CARGO";
const NO_EXIT_CODE: i32 = 127;
const BUILD_FINISHED_MESSAGE: &str = r#""build-finished""#;
const ADDITIONAL_OPTIONS: &str = "\nADDITIONAL OPTIONS:\n        --limit <NUM>                                Limit compiler messages number (0 means no limit, which is default)\n        --asc                                        Show compiler messages in ascending order";

pub fn run_cargo_filtered(cargo_command: &str) -> Result<i32> {
    let ParsedArgs {
        cargo_args,
        limit_messages,
        ascending_messages_order,
        help,
    } = ParsedArgs::parse(env::args().skip(2))?;

    let cargo_args = iter::once(cargo_command.to_owned())
        .chain(iter::once(MESSAGE_FORMAT.to_owned()))
        .chain(cargo_args);

    let cargo = env::var(CARGO_ENV_VAR)
        .map(PathBuf::from)
        .ok()
        .unwrap_or_else(|| PathBuf::from(CARGO_EXECUTABLE));

    let mut command = Command::new(cargo)
        .args(cargo_args)
        .stdout(Stdio::piped())
        .spawn()?;

    let mut reader = BufReader::new(command.stdout.take().context("cannot read stdout")?);

    if !help {
        let raw_messages = read_raw_messages(&mut reader)?;
        parse_and_process_messages(raw_messages, limit_messages, ascending_messages_order)?;
    }

    io::copy(&mut reader, &mut FlushingWriter::new(io::stdout()))?;

    if help {
        println!("{}", ADDITIONAL_OPTIONS);
    }

    let exit_code = command.wait()?.code().unwrap_or(NO_EXIT_CODE);
    Ok(exit_code)
}

fn parse_and_process_messages(
    raw_messages: Vec<u8>,
    limit_messages: usize,
    ascending_messages_order: bool,
) -> Result<()> {
    let mut internal_compiler_errors = Vec::new();
    let mut errors = Vec::new();
    let mut non_errors = Vec::new();

    for message in cargo_metadata::Message::parse_stream(Cursor::new(raw_messages)) {
        if let Message::CompilerMessage(compiler_message) = message? {
            if let Some(rendered) = compiler_message.message.rendered {
                match compiler_message.message.level {
                    DiagnosticLevel::Ice => {
                        internal_compiler_errors.push(rendered);
                    }
                    DiagnosticLevel::Error => {
                        errors.push(rendered);
                    }
                    _ => {
                        non_errors.push(rendered);
                    }
                }
            }
        }
    }

    let messages = internal_compiler_errors
        .into_iter()
        .chain(errors.into_iter())
        .chain(non_errors.into_iter());

    let no_limit = limit_messages == 0;
    let messages = if no_limit {
        Either::Left(messages)
    } else {
        Either::Right(messages.take(limit_messages))
    };

    let mut messages = messages.collect::<Vec<_>>();
    if !ascending_messages_order {
        messages.reverse();
    }

    for message in messages {
        print!("{}", message);
    }

    Ok(())
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
