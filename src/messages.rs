use crate::{
    io::Buffers,
    models::{EditorData, SourceFile},
    options::Options,
    process,
};
use anyhow::{Context, Result};
use cargo_metadata::{
    diagnostic::{DiagnosticLevel, DiagnosticSpan},
    CompilerMessage, Message,
};
use getset::CopyGetters;
use itertools::{Either, Itertools};
use process::{failed_to_execute_error_text, CargoProcess};
use std::{
    collections::HashSet,
    io::Write,
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

// TODO: split module?

#[derive(Default, CopyGetters)]
pub struct Messages {
    internal_compiler_errors: Vec<CompilerMessage>,
    errors: Vec<CompilerMessage>,
    non_errors: Vec<CompilerMessage>,

    #[get_copy = "pub"]
    child_killed: bool,
}

struct FilteredMessages {
    errors: Vec<CompilerMessage>,
    warnings: Vec<CompilerMessage>,
}

struct TransformedMessages {
    messages: Vec<Message>,
    source_files_in_consistent_order: Vec<SourceFile>,
}

// TODO: remove?
pub struct MessageProcessor;

impl Messages {
    pub fn parse_with_timeout_on_error(
        buffers: &mut Buffers,
        cargo_process: Option<&CargoProcess>,
        options: &Options,
    ) -> Result<Self> {
        let mut result = Messages::default();
        if options.help() || options.version() {
            return Ok(result);
        }

        for message in Message::parse_stream(buffers.child_stdout_reader_mut()) {
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

            if let Some(cargo_process) = cargo_process {
                if result.has_errors() {
                    let time_limit = options.time_limit_after_error();
                    if time_limit > Duration::from_secs(0) {
                        cargo_process.kill_after_timeout(time_limit); // TODO: closure?
                    }
                }
            }
        }

        result.child_killed = if let Some(cargo_process) = cargo_process {
            cargo_process.wait_if_killing_is_in_progress() == process::State::Killed
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

impl FilteredMessages {
    fn filter(messages: Messages, options: &Options, workspace_root: &Path) -> Self {
        let warnings = if options.show_dependencies_warnings() {
            messages.non_errors
        } else {
            messages
                .non_errors
                .into_iter()
                .filter(|i| i.target.src_path.starts_with(workspace_root))
                .collect()
        };

        let errors = messages
            .internal_compiler_errors
            .into_iter()
            .chain(messages.errors.into_iter())
            .collect();

        Self { errors, warnings }
    }
}

impl MessageProcessor {
    pub fn process(
        buffers: &mut Buffers,
        parsed_messages: Messages,
        options: &Options,
        workspace_root: &Path,
    ) -> Result<()> {
        let TransformedMessages {
            messages,
            source_files_in_consistent_order,
        } = TransformedMessages::transform(parsed_messages, options, workspace_root)?;

        let processed_messages = messages.into_iter();
        // TODO: closure?
        if options.json_message_format() {
            for message in processed_messages {
                buffers.writeln_to_stdout(&serde_json::to_string(&message)?)?;
            }
        } else {
            for message in processed_messages.filter_map(|message| match message {
                Message::CompilerMessage(compiler_message) => compiler_message.message.rendered,
                _ => None,
            }) {
                buffers.write_to_stderr(message)?;
            }
        }

        Self::open_affected_files_in_external_app(
            buffers,
            source_files_in_consistent_order,
            options,
            workspace_root,
        )
    }

    fn open_affected_files_in_external_app(
        buffers: &mut Buffers,
        source_files_in_consistent_order: Vec<SourceFile>,
        options: &Options,
        workspace_root: &Path,
    ) -> Result<()> {
        let app = &options.open_in_external_app();
        if !app.is_empty() {
            let editor_data = EditorData::new(workspace_root, source_files_in_consistent_order);
            // TODO: Command in messages.rs? closure?
            let mut child = Command::new(app).stdin(Stdio::piped()).spawn()?;
            child
                .stdin
                .take()
                .context("no stdin")?
                .write_all(serde_json::to_string(&editor_data)?.as_bytes())?;

            let error_text = failed_to_execute_error_text(app);
            let output = child.wait_with_output().context(error_text)?;

            buffers.write_all_to_stderr(&output.stdout)?;
            buffers.write_all_to_stderr(&output.stderr)?;
        }
        Ok(())
    }
}

impl TransformedMessages {
    fn transform(
        messages: Messages,
        options: &Options,
        workspace_root: &Path,
    ) -> Result<TransformedMessages> {
        let has_errors = messages.has_errors();
        let FilteredMessages { errors, warnings } =
            FilteredMessages::filter(messages, options, workspace_root);

        let errors = Self::filter_and_order_messages(errors, workspace_root);
        let warnings = Self::filter_and_order_messages(warnings, workspace_root);

        let messages = if options.show_warnings_if_errors_exist() {
            Either::Left(errors.chain(warnings))
        } else {
            let messages = if has_errors {
                Either::Left(errors)
            } else {
                Either::Right(warnings)
            };
            Either::Right(messages)
        };

        let limit_messages = options.limit_messages();
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
            Self::extract_source_files_for_external_app(&messages, options, workspace_root);

        let messages = messages.into_iter();
        let messages = {
            if options.ascending_messages_order() {
                Either::Left(messages)
            } else {
                Either::Right(messages.rev())
            }
        }
        .map(Message::CompilerMessage)
        .collect();

        Ok(TransformedMessages {
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
        options: &Options,
        workspace_root: &Path,
    ) -> Vec<SourceFile> {
        let spans_and_messages = messages
            .iter()
            .filter(|message| {
                if options.open_in_external_app_on_warnings() {
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
            .map(|(span, message)| (Self::find_leaf_project_expansion(span), &message.message));

        let mut source_files_in_consistent_order = Vec::new();
        let mut used_file_names = HashSet::new();
        for (span, message) in spans_and_messages {
            if !used_file_names.contains(&span.file_name) {
                used_file_names.insert(span.file_name.clone());
                source_files_in_consistent_order.push(SourceFile::new(
                    span,
                    message,
                    workspace_root,
                ));
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
}
