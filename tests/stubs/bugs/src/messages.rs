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
    pub fn f() {
        non_existent(); // NOTE
    }

    pub fn merge(&mut self, other: Self) {
        todo!()
    }
}
