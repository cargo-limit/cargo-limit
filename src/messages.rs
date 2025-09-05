use crate::{io::Buffers, models::Location, options::Options, process};
use anyhow::Result;
use cargo_metadata::{
    CompilerMessage, Message,
    diagnostic::{DiagnosticLevel, DiagnosticSpan},
};
use itertools::{Either, Itertools};
use process::CargoProcess;
use std::path::Path;

#[derive(Default, Debug)]
pub struct Messages {
    internal_compiler_errors: Vec<CompilerMessage>,
    errors: Vec<CompilerMessage>,
    non_errors: Vec<CompilerMessage>,
    pub child_killed: bool,
}

struct FilteredAndOrderedMessages {
    errors: Vec<CompilerMessage>,
    warnings: Vec<CompilerMessage>,
}

struct TransformedMessages {
    messages: Vec<Message>,
    locations_in_consistent_order: Vec<Location>,
}

pub fn transform_and_process_messages(
    buffers: &mut Buffers,
    messages: Messages,
    options: &Options,
    workspace_root: Option<&Path>,
    mut process: impl FnMut(&mut Buffers, Vec<Message>, Vec<Location>, &Path) -> Result<()>,
) -> Result<()> {
    if let Some(workspace_root) = workspace_root {
        let TransformedMessages {
            messages,
            locations_in_consistent_order,
        } = TransformedMessages::transform(messages, options, workspace_root)?;
        process(
            buffers,
            messages,
            locations_in_consistent_order,
            workspace_root,
        )?;
    }
    Ok(())
}

impl Messages {
    pub fn parse_with_timeout_on_error(
        buffers: &mut Buffers,
        cargo_process: Option<&CargoProcess>,
        options: &Options,
    ) -> Result<Self> {
        let mut result = Messages::default();
        if options.help || options.version {
            return Ok(result);
        }

        for message in buffers.map_child_stdout_reader(Message::parse_stream) {
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

            if let Some(cargo_process) = cargo_process
                && result.has_errors()
                && let Some(time_limit) = options.time_limit_after_error
            {
                cargo_process.kill_after_timeout(time_limit);
            }
        }

        result.child_killed = if let Some(cargo_process) = cargo_process {
            cargo_process.wait_if_killing_is_in_progress() == process::State::NotRunning
        } else {
            false
        };

        Ok(result)
    }

    pub fn merge(&mut self, other: Self) {
        self.internal_compiler_errors
            .extend(other.internal_compiler_errors);
        self.errors.extend(other.errors);
        self.non_errors.extend(other.non_errors);
        self.child_killed |= other.child_killed;
    }

    fn has_errors(&self) -> bool {
        !self.errors.is_empty() || !self.internal_compiler_errors.is_empty()
    }
}

impl FilteredAndOrderedMessages {
    fn filter(messages: Messages, options: &Options, workspace_root: &Path) -> Self {
        let non_errors = messages.non_errors.into_iter();
        let warnings = if options.show_dependencies_warnings {
            Either::Left(non_errors)
        } else {
            Either::Right(non_errors.filter(|i| i.target.src_path.starts_with(workspace_root)))
        };
        let warnings = Self::filter_and_order_messages(warnings, workspace_root);

        let cargo_errors = Self::filter_cargo_errors(&messages.errors);
        let errors = messages
            .internal_compiler_errors
            .into_iter()
            .chain(messages.errors);
        let errors = Self::filter_and_order_messages(errors, workspace_root);
        let errors = if errors.is_empty() {
            cargo_errors
        } else {
            errors
        };

        Self { errors, warnings }
    }

    // TODO: rename
    // TODO: rewrite with loop?
    fn filter_cargo_errors(messages: &[CompilerMessage]) -> Vec<CompilerMessage> {
        // TODO: error: could not compile `a` (lib) due to 4 previous errors
        // TODO: warning: build failed, waiting for other jobs to finish...
        // TODO: error: could not compile `a` (lib test) due to 4 previous errors
        // TODO: is it just DiagnosticLevel::FailureNote?
        // TODO: write test?
        let (good, bad): (Vec<_>, Vec<_>) = messages
            .iter()
            .filter_map(|i| {
                if i.message.spans.is_empty() && i.message.rendered.is_some() {
                    let i = i.clone();
                    let item = if i.message.message.contains("aborting due to previous error") {
                        (None, Some(i))
                    } else {
                        (Some(i), None)
                    };
                    Some(item)
                } else {
                    None
                }
            })
            .unzip();

        let filter = |items: Vec<Option<CompilerMessage>>| -> Vec<CompilerMessage> {
            items
                .into_iter()
                .flatten()
                .unique_by(|i| i.message.rendered.clone())
                .collect()
        };
        let good = filter(good);
        let bad = filter(bad);

        if good.is_empty() { bad } else { good }
    }

    fn filter_and_order_messages(
        messages: impl IntoIterator<Item = CompilerMessage>,
        workspace_root: &Path,
    ) -> Vec<CompilerMessage> {
        messages
            .into_iter()
            .flat_map(|i| {
                let key = i
                    .message
                    .spans
                    .iter()
                    .filter(|span| span.is_primary)
                    .cloned()
                    .map(TransformedMessages::find_leaf_project_expansion)
                    .map(|span| (span.file_name, span.line_start, span.column_start))
                    .min_by_key(|(_, line, column)| (*line, *column));
                Some((key?, i))
            })
            .sorted_by_key(|(key, message)| {
                let (file_name, _, _) = &key;
                let is_dependency = !message.target.src_path.starts_with(workspace_root);
                let is_relative = Path::new(&file_name).is_relative();
                (is_dependency, is_relative, key.clone())
            })
            .unique_by(|(key, _)| key.clone())
            .map(|(_, message)| message)
            .collect_vec()
    }
}

impl TransformedMessages {
    fn transform(
        messages: Messages,
        options: &Options,
        workspace_root: &Path,
    ) -> Result<TransformedMessages> {
        let FilteredAndOrderedMessages { errors, warnings } =
            FilteredAndOrderedMessages::filter(messages, options, workspace_root);
        let has_errors = !errors.is_empty();

        let errors = errors.into_iter();
        let warnings = warnings.into_iter();
        let messages = if options.show_warnings_if_errors_exist {
            Either::Left(errors.chain(warnings))
        } else {
            let messages = if has_errors {
                Either::Left(errors)
            } else {
                Either::Right(warnings)
            };
            Either::Right(messages)
        };

        let limit_messages = options.limit_messages;
        let no_limit = limit_messages == 0;
        let messages = {
            if no_limit {
                Either::Left(messages)
            } else {
                Either::Right(messages.take(limit_messages))
            }
        }
        .collect_vec();

        let locations_in_consistent_order =
            Self::extract_locations_for_external_app(&messages, options, workspace_root);

        let messages = messages.into_iter();
        let messages = {
            if options.ascending_messages_order {
                Either::Left(messages)
            } else {
                Either::Right(messages.rev())
            }
        }
        .map(Message::CompilerMessage)
        .collect();

        Ok(Self {
            messages,
            locations_in_consistent_order,
        })
    }

    fn extract_locations_for_external_app(
        messages: &[CompilerMessage],
        options: &Options,
        workspace_root: &Path,
    ) -> Vec<Location> {
        // TODO: do find_leaf_project_expansion once
        messages
            .iter()
            .filter(|message| {
                if options.open_in_external_app_on_warnings {
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
                    .map(TransformedMessages::find_leaf_project_expansion)
                    .map(|span| (span.line_start, span.column_start, span))
                    .min_by_key(|(line, column, _)| (*line, *column))
                    .map(move |(_, _, span)| (span, message))
            })
            .map(|(span, message)| Location::new(span, &message.message, workspace_root))
            .collect()
    }

    fn find_leaf_project_expansion(mut span: DiagnosticSpan) -> DiagnosticSpan {
        let mut project_span = span.clone();
        while let Some(expansion) = span.expansion {
            span = expansion.span;
            project_span = span.clone();
        }
        project_span
    }
}
