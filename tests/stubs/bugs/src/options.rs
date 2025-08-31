use crate::{cargo_toml::CargoToml, process::CARGO_EXECUTABLE};
use anyhow::{format_err, Context, Result};
use itertools::Either;
use std::{env, io, io::IsTerminal, iter, path::Path, str::FromStr, time::Duration};

const APP_ARGS_DELIMITER: &str = "--";

const COLOR_AUTO: &str = "";

#[derive(Debug, PartialEq)]
pub struct Options {
    cargo_args: Vec<String>,
    args_after_app_args_delimiter: Vec<String>,
    terminal_supports_colors: bool,

    pub limit_messages: usize,
    pub time_limit_after_error: Duration,
    pub ascending_messages_order: bool,
    pub show_warnings_if_errors_exist: bool,
    pub show_dependencies_warnings: bool,
    pub open_in_external_app: String,
    pub open_in_external_app_on_warnings: bool,
    pub help: bool,
    pub version: bool,
    pub json_message_format: bool,
    short_message_format: bool,
}

#[derive(Debug, PartialEq)]
struct ParsedSubcommand {
    subcommand: String,
    open_in_external_app_on_warnings: bool,
    remaining_args: Vec<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            cargo_args: Vec::new(),
            args_after_app_args_delimiter: Vec::new(),
            terminal_supports_colors: true,
            limit_messages: 0,
            time_limit_after_error: Some(Duration::from_secs(1)), // NOTE
            ascending_messages_order: false,
            show_warnings_if_errors_exist: false,
            show_dependencies_warnings: false,
            open_in_external_app: "_cargo-limit-open-in-nvim".to_owned(),
            open_in_external_app_on_warnings: false,
            help: false,
            version: false,
            json_message_format: false,
            short_message_format: false,
        }
    }
}

impl Options {
    pub fn all_args(&self) -> impl Iterator<Item = String> {
        let delimiter = if self.args_after_app_args_delimiter.is_empty() {
            Either::Left(iter::empty())
        } else {
            Either::Right(iter::once(APP_ARGS_DELIMITER.to_owned()))
        };
        self.cargo_args
            .clone()
            .into_iter()
            .chain(delimiter)
            .chain(self.args_after_app_args_delimiter.clone())
    }

    fn from_vars_and_atty() -> Result<Self> {
        let mut result = Self::default();
        {
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
        }
        Ok(result)
    }
}
