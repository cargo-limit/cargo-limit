use crate::{cargo_toml::CargoToml, process::CARGO_EXECUTABLE};
use anyhow::{format_err, Context, Result};
use itertools::Either;
use std::{env, io, io::IsTerminal, iter, path::Path, str::FromStr, time::Duration};

#[derive(Debug, PartialEq)]
pub struct Options {
    pub time_limit_after_error: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            time_limit_after_error: Some(Duration::from_secs(1)), // NOTE
        }
    }
}

impl Options {
    fn from_vars_and_atty() -> Result<Self> {
        let mut result = Self::default();
        let mut seconds = result
            .time_limit_after_error
            .map(Duration::as_secs) // NOTE
            .unwrap_or(0);
        let duration = Duration::from_secs(seconds);
        result.time_limit_after_error = if duration > Duration::from_secs(0) {
            Some(duration) // NOTE
        } else {
            None // NOTE
        };
        Ok(result)
    }
}
