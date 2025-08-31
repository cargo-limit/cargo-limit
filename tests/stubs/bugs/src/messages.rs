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
    pub child_killed: bool,
}

impl Messages {
    pub fn parse_with_timeout_on_error(
        buffers: &mut Buffers,
        cargo_process: Option<&CargoProcess>,
        options: &Options,
    ) -> Result<Self> {
        let mut result = Messages::default();
        non_existent(); // NOTE
        Ok(result)
    }

    pub fn merge(&mut self, other: Self) {
        todo!()
    }
}
