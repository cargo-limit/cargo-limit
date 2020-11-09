use crate::{options::Options, process};
use anyhow::Result;
use cargo_metadata::{diagnostic::DiagnosticLevel, CompilerMessage, Message, MetadataCommand};
use either::Either;
use itertools::Itertools;
use std::{
    io::{self, Cursor},
    thread,
    time::Duration,
};

const BUILD_FINISHED_MESSAGE: &str = r#""build-finished""#;
const ERROR_MESSAGE: &str = r#""level":"error""#;

#[derive(Default)]
pub struct ParsedMessages {
    internal_compiler_errors: Vec<CompilerMessage>,
    errors: Vec<CompilerMessage>,
    non_errors: Vec<CompilerMessage>,
}

pub struct RawMessages {
    pub jsons: Vec<u8>,
    pub others: Vec<String>,
}

impl ParsedMessages {
    pub fn parse(raw_messages: Vec<u8>) -> Result<Self> {
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

pub fn process_messages(
    parsed_messages: ParsedMessages,
    parsed_args: &Options,
) -> Result<impl Iterator<Item = Message>> {
    let non_errors = if parsed_args.show_dependencies_warnings {
        Either::Left(parsed_messages.non_errors.into_iter())
    } else {
        let workspace_root = MetadataCommand::new().exec()?.workspace_root;
        let non_errors = parsed_messages
            .non_errors
            .into_iter()
            .filter(|i| i.target.src_path.starts_with(&workspace_root));
        Either::Right(non_errors.collect::<Vec<_>>().into_iter())
    };

    let messages = if parsed_args.show_warnings_if_errors_exist {
        Either::Left(
            parsed_messages
                .internal_compiler_errors
                .into_iter()
                .chain(parsed_messages.errors.into_iter())
                .chain(non_errors),
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
            Either::Right(non_errors)
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

    let messages = messages.map(Message::CompilerMessage);
    Ok(messages)
}

impl RawMessages {
    pub fn read<R: io::BufRead>(
        reader: &mut R,
        cargo_pid: u32,
        parsed_args: &Options,
    ) -> Result<RawMessages> {
        let mut line = String::new();
        let mut jsons = Vec::new();
        let mut others = Vec::new();
        let mut kill_timer_handler = None;

        loop {
            let len = reader.read_line(&mut line)?;

            if len == 0 || line.contains(BUILD_FINISHED_MESSAGE) {
                break;
            } else if line.starts_with('{') {
                if line.contains(ERROR_MESSAGE) {
                    let time_limit = parsed_args.time_limit_after_error;
                    if time_limit > Duration::from_secs(0) && kill_timer_handler.is_none() {
                        kill_timer_handler = Some(thread::spawn(move || {
                            thread::sleep(time_limit);
                            process::kill(cargo_pid)
                        }));
                    }
                }
                jsons.extend(line.as_bytes());
            } else {
                others.push(line.clone());
            }

            line.clear();
        }

        if let Some(kill_timer_handler) = kill_timer_handler {
            kill_timer_handler
                .join()
                .expect("kill timer thread panicked");
        }

        Ok(Self { jsons, others })
    }
}
