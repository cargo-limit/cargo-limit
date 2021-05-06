use anyhow::Result;
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
        let mut cargo_toml_path = workspace_root.to_path_buf();
        cargo_toml_path.push("Cargo.toml");
        Ok(toml::from_str(&String::from_utf8(fs::read(
            cargo_toml_path,
        )?)?)?)
    }

    pub fn all_tests_have_harness(&self) -> bool {
        Self::all_have_harness(self.test.iter())
    }

    pub fn all_benchmarks_have_harness(&self) -> bool {
        Self::all_have_harness(self.bench.iter())
    }

    fn all_have_harness<'i>(items: impl Iterator<Item = &'i Item>) -> bool {
        items.map(|i| i.harness).flatten().all(identity)
    }
}
