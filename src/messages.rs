use crate::{options::Options, process};
use anyhow::Result;
use cargo_metadata::{diagnostic::DiagnosticLevel, CompilerMessage, Message, MetadataCommand};
use either::Either;
use itertools::Itertools;
use std::{io, time::Duration};

#[derive(Default)]
pub struct ParsedMessages {
    internal_compiler_errors: Vec<CompilerMessage>,
    errors: Vec<CompilerMessage>,
    non_errors: Vec<CompilerMessage>,
}

impl ParsedMessages {
    pub fn parse<R: io::BufRead>(
        reader: &mut R,
        cargo_pid: u32,
        parsed_args: &Options,
    ) -> Result<Self> {
        let mut result = ParsedMessages::default();
        let mut kill_timer_started = false;

        for message in Message::parse_stream(reader) {
            match message? {
                Message::CompilerMessage(compiler_message) => {
                    match compiler_message.message.level {
                        DiagnosticLevel::Ice => {
                            result.internal_compiler_errors.push(compiler_message)
                        },
                        DiagnosticLevel::Error => result.errors.push(compiler_message),
                        _ => result.non_errors.push(compiler_message),
                    }
                },
                Message::BuildFinished(_) => {
                    break;
                },
                _ => (),
            }

            if !result.errors.is_empty() || !result.internal_compiler_errors.is_empty() {
                let time_limit = parsed_args.time_limit_after_error;
                if time_limit > Duration::from_secs(0) && !kill_timer_started {
                    kill_timer_started = true;
                    process::wait_in_background_and_kill_and_print(cargo_pid, time_limit);
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
