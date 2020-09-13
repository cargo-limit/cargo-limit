mod flushing_writer;
mod parsed_args;

use anyhow::{Context, Result};
use cargo_metadata::{diagnostic::DiagnosticLevel, Message};
use flushing_writer::FlushingWriter;
use parsed_args::ParsedArgs;
use std::{
    env,
    io::{self, BufRead, BufReader, Cursor},
    path::PathBuf,
    process::{Command, Stdio},
};

pub const MESSAGE_FORMAT: &str = "--message-format=json-diagnostic-rendered-ansi";
pub const BENCH: &str = "bench";
pub const BUILD: &str = "build";
pub const RUN: &str = "run";
pub const TEST: &str = "test";

const CARGO_EXECUTABLE: &str = "cargo";
const CARGO_ENV_VAR: &str = "CARGO";
const NO_EXIT_CODE: i32 = 127;
const BUILD_FINISHED_MESSAGE: &str = r#""build-finished""#;
const ADDITIONAL_OPTIONS: &str = "\nADDITIONAL OPTIONS:\n        --limit-messages <NUM>                       Limit number of compiler messages";

pub fn run_cargo_filtered(first_cargo_args: &[&str]) -> Result<i32> {
    let ParsedArgs {
        cargo_args,
        limit_messages,
        help,
    } = ParsedArgs::parse(env::args().skip(2))?;

    let cargo_args = first_cargo_args
        .iter()
        .map(|i| (*i).to_owned())
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
        parse_and_process_messages(raw_messages, limit_messages)?;
    }

    io::copy(&mut reader, &mut FlushingWriter::new(std::io::stdout()))?;

    if help {
        println!("{}", ADDITIONAL_OPTIONS);
    }

    let exit_code = command.wait()?.code().unwrap_or(NO_EXIT_CODE);
    Ok(exit_code)
}

fn parse_and_process_messages(raw_messages: Vec<u8>, limit_messages: usize) -> Result<()> {
    let mut errors = Vec::new();
    let mut non_errors = Vec::new();

    for message in cargo_metadata::Message::parse_stream(Cursor::new(raw_messages)) {
        match message? {
            Message::CompilerMessage(compiler_message) => {
                if let Some(rendered) = compiler_message.message.rendered {
                    match compiler_message.message.level {
                        DiagnosticLevel::Error | DiagnosticLevel::Ice => {
                            errors.push(rendered);
                        }
                        _ => {
                            non_errors.push(rendered);
                        }
                    }
                }
            }
            _ => (),
        }
    }

    for message in errors
        .into_iter()
        .chain(non_errors.into_iter())
        .take(limit_messages)
    {
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
