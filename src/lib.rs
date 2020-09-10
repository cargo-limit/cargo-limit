use anyhow::{Context, Result};
use cargo_metadata::{diagnostic::DiagnosticLevel, Message};
use std::{
    env,
    ffi::OsStr,
    io::BufReader,
    iter,
    process::{Command, Stdio},
};
use terminal_size::{terminal_size, Width};

pub const MESSAGE_FORMAT: &str = "--message-format=json-diagnostic-rendered-ansi";
pub const NO_RUN: &str = "--no-run";

pub const BENCH: &str = "bench";
pub const BUILD: &str = "build";
pub const RUN: &str = "run";
pub const TEST: &str = "test";

const CARGO: &str = "cargo";
const NO_EXIT_CODE: i32 = 127;

fn clear_current_line() {
    if let Some((Width(width), _)) = terminal_size() {
        let spaces = iter::repeat(' ').take(width as usize).collect::<String>();
        print!("{}\r", spaces);
    }
}

pub fn run_cargo<I, S>(args: I) -> Result<i32>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new(CARGO).args(args).spawn()?;
    let exit_code = command.wait()?.code().unwrap_or(NO_EXIT_CODE);
    Ok(exit_code)
}

pub fn run_cargo_filtered<I, S>(args: I, limit: usize, allow_non_errors: bool) -> Result<i32>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new(CARGO)
        .args(args)
        .stdout(Stdio::piped())
        .spawn()?;

    let mut errors = Vec::new();
    let mut non_errors = Vec::new();

    let reader = BufReader::new(command.stdout.take().context("cannot read stdout")?);
    for message in cargo_metadata::Message::parse_stream(reader) {
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

    if errors.is_empty() && allow_non_errors {
        for message in non_errors.into_iter() {
            clear_current_line();
            print!("{}", message);
        }
    } else {
        for message in errors.into_iter().take(limit) {
            clear_current_line();
            print!("{}", message);
        }
    }

    let exit_code = command.wait()?.code().unwrap_or(NO_EXIT_CODE);
    Ok(exit_code)
}

pub fn prepare_args<'a>(args: &'a [&str]) -> impl Iterator<Item = String> + 'a {
    let passed_cargo_args = env::args().skip(2);
    args.iter()
        .map(|i| (*i).to_owned())
        .chain(passed_cargo_args)
}
