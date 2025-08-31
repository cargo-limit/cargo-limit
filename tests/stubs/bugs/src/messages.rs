use crate::{io::Buffers, models::Location, options::Options, process};
use anyhow::Result;
use cargo_metadata::{
    diagnostic::{DiagnosticLevel, DiagnosticSpan},
    CompilerMessage, Message,
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
    workspace_root: &Path,
    mut process: impl FnMut(&mut Buffers, Vec<Message>, Vec<Location>) -> Result<()>,
) -> Result<()> {
    let TransformedMessages {
        messages,
        locations_in_consistent_order,
    } = TransformedMessages::transform(messages, options, workspace_root)?;
    process(buffers, messages, locations_in_consistent_order)
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

        non_existent();
        Ok(result)
    }

    pub fn merge(&mut self, other: Self) {
        todo!()
    }

    fn has_errors(&self) -> bool {
        todo!()
    }
}

impl FilteredAndOrderedMessages {
    fn filter(messages: Messages, options: &Options, workspace_root: &Path) -> Self {
        todo!()
    }

    fn filter_cargo_errors(messages: &[CompilerMessage]) -> Vec<CompilerMessage> {
        todo!()
    }

    fn filter_and_order_messages(
        messages: impl IntoIterator<Item = CompilerMessage>,
        workspace_root: &Path,
    ) -> Vec<CompilerMessage> {
        todo!()
    }
}

impl TransformedMessages {
    fn transform(
        messages: Messages,
        options: &Options,
        workspace_root: &Path,
    ) -> Result<TransformedMessages> {
        todo!()
    }

    fn extract_locations_for_external_app(
        messages: &[CompilerMessage],
        options: &Options,
        workspace_root: &Path,
    ) -> Vec<Location> {
        todo!()
    }

    fn find_leaf_project_expansion(mut span: DiagnosticSpan) -> DiagnosticSpan {
        todo!()
    }
}
