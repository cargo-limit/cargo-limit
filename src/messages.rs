use crate::{models::SourceFile, options::Options, process};
use anyhow::Result;
use cargo_metadata::{
    diagnostic::{DiagnosticLevel, DiagnosticSpan},
    CompilerMessage, Message,
};
use itertools::{Either, Itertools};
use std::{collections::HashSet, io, io::Write, path::Path, time::Duration};

// TODO: Default? pub?
#[derive(Default)]
pub struct ParsedMessages {
    internal_compiler_errors: Vec<CompilerMessage>,
    errors: Vec<CompilerMessage>,
    non_errors: Vec<CompilerMessage>,
}

struct ErrorsAndWarnings {
    errors: Vec<CompilerMessage>,
    warnings: Vec<CompilerMessage>,
}

pub struct ProcessedMessages {
    pub messages: Vec<Message>,
    pub source_files_in_consistent_order: Vec<SourceFile>,
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
                    process::wait_in_background_and_kill(cargo_pid, time_limit, move || {
                        let _ = std::writeln!(&mut io::stdout(), "");
                    });
                }
            }
        }

        Ok(result)
    }
}

impl ErrorsAndWarnings {
    fn process(
        parsed_messages: ParsedMessages,
        parsed_args: &Options,
        workspace_root: &Path,
    ) -> Self {
        let warnings = if parsed_args.show_dependencies_warnings {
            parsed_messages.non_errors
        } else {
            parsed_messages
                .non_errors
                .into_iter()
                .filter(|i| i.target.src_path.starts_with(workspace_root))
                .collect()
        };

        let errors = parsed_messages
            .internal_compiler_errors
            .into_iter()
            .chain(parsed_messages.errors.into_iter())
            .collect();

        Self { errors, warnings }
    }
}

pub fn process_messages(
    parsed_messages: ParsedMessages,
    parsed_args: &Options,
    workspace_root: &Path,
) -> Result<ProcessedMessages> {
    let has_warnings_only =
        parsed_messages.internal_compiler_errors.is_empty() && parsed_messages.errors.is_empty();

    let ErrorsAndWarnings { errors, warnings } =
        ErrorsAndWarnings::process(parsed_messages, parsed_args, workspace_root);

    let errors = filter_and_order_messages(errors, workspace_root);
    let warnings = filter_and_order_messages(warnings, workspace_root);

    let messages = if parsed_args.show_warnings_if_errors_exist {
        Either::Left(errors.chain(warnings))
    } else {
        let messages = if has_warnings_only {
            Either::Left(warnings)
        } else {
            Either::Right(errors)
        };
        Either::Right(messages)
    };

    let limit_messages = parsed_args.limit_messages;
    let no_limit = limit_messages == 0;
    let messages = {
        if no_limit {
            Either::Left(messages)
        } else {
            Either::Right(messages.take(limit_messages))
        }
    }
    .collect::<Vec<_>>();

    let source_files_in_consistent_order =
        extract_source_files_for_external_app(&messages, parsed_args, workspace_root);

    let messages = messages.into_iter();
    let messages = {
        if parsed_args.ascending_messages_order {
            Either::Left(messages)
        } else {
            Either::Right(messages.rev())
        }
    }
    .map(Message::CompilerMessage)
    .collect();

    Ok(ProcessedMessages {
        messages,
        source_files_in_consistent_order,
    })
}

fn filter_and_order_messages(
    messages: impl IntoIterator<Item = CompilerMessage>,
    workspace_root: &Path,
) -> impl Iterator<Item = CompilerMessage> {
    let messages = messages
        .into_iter()
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

    let mut project_messages = Vec::new();
    let mut dependencies_messages = Vec::new();
    for i in messages {
        if i.target.src_path.starts_with(workspace_root) {
            project_messages.push(i);
        } else {
            dependencies_messages.push(i);
        }
    }

    project_messages.into_iter().chain(dependencies_messages)
}

fn extract_source_files_for_external_app(
    messages: &[CompilerMessage],
    parsed_args: &Options,
    workspace_root: &Path,
) -> Vec<SourceFile> {
    let spans_and_messages = messages
        .iter()
        .filter(|message| {
            if parsed_args.open_in_external_app_on_warnings {
                true
            } else {
                matches!(
                    message.message.level,
                    DiagnosticLevel::Error | DiagnosticLevel::Ice
                )
            }
        })
        .flat_map(|message| {
            message
                .message
                .spans
                .iter()
                .filter(|span| span.is_primary)
                .cloned()
                .map(move |span| (span, message))
        })
        .map(|(span, message)| (find_leaf_project_expansion(span), &message.message));

    let mut source_files_in_consistent_order = Vec::new();
    let mut used_file_names = HashSet::new();
    for (span, message) in spans_and_messages {
        if !used_file_names.contains(&span.file_name) {
            used_file_names.insert(span.file_name.clone());
            source_files_in_consistent_order.push(SourceFile::new(span, message, workspace_root));
        }
    }

    source_files_in_consistent_order
}

fn find_leaf_project_expansion(mut span: DiagnosticSpan) -> DiagnosticSpan {
    let mut project_span = span.clone();
    while let Some(expansion) = span.expansion {
        span = expansion.span;
        if Path::new(&span.file_name).is_relative() {
            project_span = span.clone();
        }
    }
    project_span
}
