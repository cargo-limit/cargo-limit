use anyhow::{Context, Result};
use cargo_metadata::diagnostic::DiagnosticLevel;
use cargo_metadata::Message;
use std::iter;
use std::process::{exit, Command, Stdio};
use terminal_size::{terminal_size, Width};

const NO_STATUS_CODE: i32 = 127;

fn clear_current_line() {
    if let Some((Width(width), _)) = terminal_size() {
        let spaces = iter::repeat(' ').take(width as usize).collect::<String>();
        print!("{}\r", spaces);
    }
}

fn main() -> Result<()> {
    let mut command = Command::new("cargo")
        /*.args(&[
            "test",
            "--no-run",
            "--message-format=json-diagnostic-rendered-ansi",
        ])*/
        .args(&["build", "--message-format=json-diagnostic-rendered-ansi"])
        .stdout(Stdio::piped())
        .spawn()?;

    let mut important_messages = Vec::new();
    let mut boring_messages = Vec::new();

    let reader = std::io::BufReader::new(command.stdout.take().context("cannot read stdout")?);
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

    let limit = 1;
    if important_messages.is_empty() {
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

    let status_code = command.wait()?.code().unwrap_or(NO_STATUS_CODE);
    exit(status_code);
}
