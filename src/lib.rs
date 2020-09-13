mod flushing_writer;

use anyhow::{Context, Result};
use cargo_metadata::{diagnostic::DiagnosticLevel, Message};
use flushing_writer::FlushingWriter;
use std::{
    env,
    ffi::OsStr,
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

pub fn run_cargo_filtered<I, S>(args: I, limit_messages: usize) -> Result<i32>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let cargo = env::var(CARGO_ENV_VAR)
        .map(PathBuf::from)
        .ok()
        .unwrap_or_else(|| PathBuf::from(CARGO_EXECUTABLE));

    let mut command = Command::new(cargo)
        .args(args)
        .stdout(Stdio::piped())
        .spawn()?;

    let mut errors = Vec::new();
    let mut non_errors = Vec::new();

    let mut reader = BufReader::new(command.stdout.take().context("cannot read stdout")?);
    let raw_messages = read_raw_messages(&mut reader)?;

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

    io::copy(&mut reader, &mut FlushingWriter::new(std::io::stdout()))?;

    let exit_code = command.wait()?.code().unwrap_or(NO_EXIT_CODE);
    Ok(exit_code)
}

pub fn prepare_args<'a>(args: &'a [&str]) -> impl Iterator<Item = String> + 'a {
    let passed_cargo_args = env::args().skip(2);
    args.iter()
        .map(|i| (*i).to_owned())
        .chain(passed_cargo_args)
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
