mod flushing_writer;
mod messages;
mod options;

use anyhow::{Context, Result};
use cargo_metadata::Message;
use flushing_writer::FlushingWriter;
use messages::{process_messages, ParsedMessages, RawMessages};
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
    "Additional environment variables:\n    CARGO_LIMIT         Limit compiler messages number (0 \
     means no limit, which is default)\n    CARGO_ASC           Show compiler messages in \
     ascending order (false is default)\n    CARGO_FORCE_WARN    Show warnings even if errors \
     still exist (false is default)";

#[doc(hidden)]
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
    let mut stdout = FlushingWriter::new(io::stdout());
    let help = parsed_args.help;

    if !help {
        let RawMessages { jsons, others } = RawMessages::read(&mut reader)?;

        for line in others {
            stdout.write_all(line.as_bytes())?;
        }

        let parsed_messages = ParsedMessages::parse(jsons)?;
        let processed_messages = process_messages(parsed_messages, &parsed_args)?;
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

    io::copy(&mut reader, &mut stdout)?;

    if help {
        println!("\n{}", ADDITIONAL_ENVIRONMENT_VARIABLES);
    }

    let exit_code = command.wait()?.code().unwrap_or(NO_EXIT_CODE);
    Ok(exit_code)
}
