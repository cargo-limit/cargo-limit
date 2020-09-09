use anyhow::{Context, Result};
use cargo_metadata::{diagnostic::DiagnosticLevel, Message};
use std::process::{Command, Stdio};
use std::{ffi::OsStr, io::BufReader, iter};
use terminal_size::{terminal_size, Width};

pub const MESSAGE_FORMAT: &str = "--message-format=json-diagnostic-rendered-ansi";
pub const NO_RUN: &str = "--no-run";

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

pub fn run_cargo_filtered<I, S>(args: I, limit: usize, allow_boring_messages: bool) -> Result<i32>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    // TODO: env
    let mut command = Command::new(CARGO)
        .args(args)
        .stdout(Stdio::piped())
        .spawn()?;

    let mut important_messages = Vec::new();
    let mut boring_messages = Vec::new();

    let reader = BufReader::new(command.stdout.take().context("cannot read stdout")?);
    for message in cargo_metadata::Message::parse_stream(reader) {
        match message? {
            Message::CompilerMessage(msg) => {
                if let Some(rendered) = msg.message.rendered {
                    match msg.message.level {
                        DiagnosticLevel::Error | DiagnosticLevel::Ice => {
                            important_messages.push(rendered);
                        }
                        _ => {
                            boring_messages.push(rendered);
                        }
                    }
                }
            }
            _ => (),
        }
    }

    if important_messages.is_empty() && allow_boring_messages {
        for message in boring_messages.into_iter().take(limit) {
            clear_current_line();
            print!("{}", message);
        }
    } else {
        for message in important_messages.into_iter().take(limit) {
            clear_current_line();
            print!("{}", message);
        }
    }

    let exit_code = command.wait()?.code().unwrap_or(NO_EXIT_CODE);
    Ok(exit_code)
}
