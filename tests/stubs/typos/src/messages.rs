use crate::{io::Buffers, models::Location, options::Options, process};
use anyhow::Result;
use cargo_metadata::{
    diagnostic::{DiagnosticLevel, DiagnosticSpan},
    CompilerMessage, Message,
};
use getset::CopyGetters;
use itertools::{Either, Itertools};
use process::CargoProcess;
use std::{collections::HashSet, path::Path, time::Duration};

#[derive(Default, CopyGetters)]
pub struct Messages {
    internal_compiler_errors: Vec<CompilerMessage>,
    errors: Vec<CompilerMessage>,
    non_errors: Vec<CompilerMessage>,

    #[get_copy = "pub"]
    child_killed: bool,
}

struct FilteredAndOrderedMessages {
    errors: Vec<CompilerMessage>,
    warnings: Vec<CompilerMessage>,
}

struct TransformedMessages {
    messages: Vec<Message>,
    locations_in_consistent_order: Vec<Location>,
}

impl Messages {
    pub fn parse_with_timeout_on_error(
        cargo_process: Option<&CargoProcess>,
        options: &Options,
    ) -> Result<Self> {
        let mut result = Messages::default();
        if options.help() || options.version() {
            return Ok(result);
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

impl FilteredAndOrderedMessages {
    fn filter(messages: Messages, options: &Options, workspace_root: &Path) -> Self {
        let non_errors = messages.non_errors.into_iter();
        let warnings = if options.show_dependencies_warnings() {
            Either::Left(non_errors)
        } else {
            Either::Right(non_errors.filter(|i| i.target.src_path.starts_with(workspace_root)))
        };
        let warnings = Self::filter_and_order_messages(warnings, workspace_root);

        let errors = messages
            .internal_compiler_errors
            .into_iter()
            .chain(messages.errors);
        let errors = Self::filter_and_order_messages(errors, workspace_root);

        Self { errors, warnings }
    }

    fn filter_and_order_messages(
        messages: impl IntoIterator<Item = CompilerMessage>,
        workspace_root: &Path,
    ) -> Vec<CompilerMessage> {
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

        project_messages
            .into_iter()
            .chain(dependencies_messages)
            .collect()
    }
}

impl TransformedMessages {
    fn transform(
        messages: Messages,
        options: &Options,
        workspace_root: &Path,
    ) -> Result<TransformedMessages> {
        let locations_in_consistent_order =
            Self::extract_locations_for_external_app(&messages, options, workspace_root);
        Ok1(TransformedMessages {
            messages: todo!(),
            locations_in_consistent_order,
        })
    }
}
