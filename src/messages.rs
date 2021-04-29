use crate::{options::Options, process};
use anyhow::Result;
use cargo_metadata::{
    diagnostic::{DiagnosticLevel, DiagnosticSpan},
    CompilerMessage, Message, MetadataCommand,
};
use itertools::{Either, Itertools};
use std::{collections::HashSet, io, time::Duration};

#[derive(Default)]
pub struct ParsedMessages {
    internal_compiler_errors: Vec<CompilerMessage>,
    errors: Vec<CompilerMessage>,
    non_errors: Vec<CompilerMessage>,
}

pub struct ProcessedMessages {
    pub messages: Vec<Message>,
    pub spans_in_consistent_order: Vec<DiagnosticSpan>,
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
) -> Result<ProcessedMessages> {
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

    let messages = messages
        .unique()
        .filter(|i| !i.message.spans.is_empty())
        .map(|i| {
            let key = i
                .message
                .spans
                .iter()
                .map(|span| (span.file_name.clone(), span.line_start))
                .collect::<Vec<_>>();
            (key, i)
        })
        .into_group_map()
        .into_iter()
        .sorted_by_key(|(paths, _messages)| paths.clone())
        .flat_map(|(_paths, messages)| messages.into_iter());

    let limit_messages = parsed_args.limit_messages;
    let no_limit = limit_messages == 0;
    let messages = if no_limit {
        Either::Left(messages)
    } else {
        Either::Right(messages.take(limit_messages))
    };

    let messages = messages.collect::<Vec<_>>();

    let spans_in_consistent_order = extract_spans_for_external_application(&messages, parsed_args);

    let messages = messages.into_iter();
    let messages = if parsed_args.ascending_messages_order {
        Either::Left(messages)
    } else {
        Either::Right(messages.rev())
    };

    let messages = messages.map(Message::CompilerMessage).collect();
    Ok(ProcessedMessages {
        messages,
        spans_in_consistent_order,
    })
}

fn extract_spans_for_external_application(
    messages: &[CompilerMessage],
    parsed_args: &Options,
) -> Vec<DiagnosticSpan> {
    let spans_for_external_application = messages
        .iter()
        .filter(|message| {
            if parsed_args.open_in_external_application_on_warnings {
                true
            } else {
                match message.message.level {
                    DiagnosticLevel::Error | DiagnosticLevel::Ice => true,
                    _ => false,
                }
            }
        })
        .flat_map(|message| {
            message
                .message
                .spans
                .iter()
                .filter(|span| span.is_primary)
                .cloned()
        });

    let mut spans_in_consistent_order = Vec::new();
    let mut used_file_names = HashSet::new();
    for span in spans_for_external_application {
        if !used_file_names.contains(&span.file_name) {
            used_file_names.insert(span.file_name.clone());
            spans_in_consistent_order.push(span);
        }
    }

    spans_in_consistent_order
}
