/*use anyhow::Result;
use serde::Deserialize;
use std::{convert::identity, fs, path::Path};

#[derive(Deserialize)]
pub struct CargoToml {
    #[serde(default)]
    test: Vec<Item>,
    #[serde(default)]
    bench: Vec<Item>,
}

#[derive(Deserialize)]
struct Item {
    #[serde(default)]
    harness: Option<bool>,
}

impl CargoToml {
    pub fn parse(workspace_root: &Path) -> Result<Self> {
        todo!()
    }

    pub fn all_tests_have_harness(&self) -> bool {
        todo!()
    }

    pub fn all_benchmarks_have_harness(&self) -> bool {
        todo!()
    }

    fn all_have_harness<'i>(items: impl Iterator<Item = &'i Item>) -> bool {
        todo!()
    }
}*/
