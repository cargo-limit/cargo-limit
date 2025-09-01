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
