use crate::options::Options;
use anyhow::Result;
use cargo_metadata::{diagnostic::DiagnosticLevel, CompilerMessage, Message};
use either::Either;
use itertools::Itertools;
use std::io::{self, BufRead, BufReader, Cursor};

const BUILD_FINISHED_MESSAGE: &str = r#""build-finished""#;

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

impl RawMessages {
    pub fn read<R: io::Read>(reader: &mut BufReader<R>) -> Result<RawMessages> {
        let mut line = String::new();
        let mut jsons = Vec::new();
        let mut others = Vec::new();

        loop {
            let len = reader.read_line(&mut line)?;

            if len == 0 || line.contains(BUILD_FINISHED_MESSAGE) {
                break;
            } else if line.starts_with('{') {
                jsons.extend(line.as_bytes());
            } else {
                others.push(line.clone());
            }

            line.clear();
        }

        Ok(Self { jsons, others })
    }
}
