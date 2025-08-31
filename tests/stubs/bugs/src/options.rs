use crate::{cargo_toml::CargoToml, process::CARGO_EXECUTABLE};
use anyhow::{format_err, Context, Result};
use const_format::concatcp;
use itertools::Either;
use std::{env, io, io::IsTerminal, iter, path::Path, str::FromStr, time::Duration};

const EXECUTABLE_PREFIX: &str = concatcp!(CARGO_EXECUTABLE, "-l");

const APP_ARGS_DELIMITER: &str = "--";

const MESSAGE_FORMAT: &str = "--message-format=";
const MESSAGE_FORMAT_JSON: &str = concatcp!(MESSAGE_FORMAT, JSON_FORMAT);
const MESSAGE_FORMAT_JSON_WITH_COLORS: &str = concatcp!(MESSAGE_FORMAT, JSON_FORMAT_WITH_COLORS);
const MESSAGE_FORMAT_JSON_SHORT: &str = concatcp!(MESSAGE_FORMAT, JSON_FORMAT_SHORT);

const JSON_FORMAT: &str = "json";
const JSON_FORMAT_WITH_COLORS: &str = "json-diagnostic-rendered-ansi";
const JSON_FORMAT_SHORT: &str = "json-diagnostic-short";
const SHORT_FORMAT: &str = "short";
const HUMAN_FORMAT: &str = "human";
const VALID_MESSAGE_FORMATS: &[&str] = &[
    HUMAN_FORMAT,
    SHORT_FORMAT,
    JSON_FORMAT,
    JSON_FORMAT_SHORT,
    JSON_FORMAT_WITH_COLORS,
    JSON_FORMAT_SHORT,
];

const COLOR: &str = "--color=";
const COLOR_AUTO: &str = "auto";
const COLOR_ALWAYS: &str = "always";
const COLOR_NEVER: &str = "never";
const VALID_COLORS: &[&str] = &[COLOR_AUTO, COLOR_ALWAYS, COLOR_NEVER];

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
            time_limit_after_error: Some(Duration::from_secs(1)),
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

    pub fn from_os_env(current_exe: String, workspace_root: &Path) -> Result<Self> {
        Self::from_vars_and_atty()?.process_args(current_exe, env::args(), workspace_root)
    }

    // TODO: rename
    fn from_vars_and_atty() -> Result<Self> {
        let mut result = Self::default();
        result.detect_terminal_color_support();

        {
            let mut seconds = result
                .time_limit_after_error
                .map(Duration::as_secs)
                .unwrap_or(0);
            let duration = Duration::from_secs(seconds);
            result.time_limit_after_error = if duration > Duration::from_secs(0) {
                Some(duration)
            } else {
                None
            };
        }

        Ok(result)
    }

    fn detect_terminal_color_support(&mut self) {
        todo!()
    }

    fn process_args(
        mut self,
        current_exe: String,
        args: impl Iterator<Item = String>,
        workspace_root: &Path,
    ) -> Result<Self> {
        let ParsedSubcommand {
            subcommand,
            open_in_external_app_on_warnings,
            remaining_args,
        } = ParsedSubcommand::parse(args, current_exe)?;
        self.open_in_external_app_on_warnings = open_in_external_app_on_warnings;

        let mut args = remaining_args.into_iter();
        self.cargo_args.push(subcommand.clone());

        let mut color = COLOR_AUTO.to_owned();
        let mut app_args_started = false;
        let mut args_before_app_args_delimiter = Vec::new();

        self.parse_options(
            &mut args,
            &mut color,
            &mut args_before_app_args_delimiter,
            &mut app_args_started,
        )?;
        self.cargo_args.push(self.message_format(color).to_owned());
        self.cargo_args.extend(args_before_app_args_delimiter);

        let mut app_color_is_set = false;
        if app_args_started {
            self.process_args_after_app_args_delimiter(args, &mut app_color_is_set);
        }

        self.process_custom_runners(subcommand, app_color_is_set, workspace_root)?;

        Ok(self)
    }

    fn parse_options(
        &mut self,
        passed_args: &mut impl Iterator<Item = String>,
        color: &mut String,
        args_before_app_args_delimiter: &mut Vec<String>,
        app_args_started: &mut bool,
    ) -> Result<()> {
        todo!()
    }

    fn message_format(&self, color: String) -> &str {
        todo!()
    }

    fn process_custom_runners(
        &mut self,
        subcommand: String,
        app_color_is_set: bool,
        workspace_root: &Path,
    ) -> Result<()> {
        todo!()
    }

    fn process_args_after_app_args_delimiter(
        &mut self,
        passed_args: impl Iterator<Item = String>,
        app_color_is_set: &mut bool,
    ) {
        todo!()
    }

    fn parse_var<T: FromStr>(key: &str, value: &mut T) -> Result<()>
    where
        <T as FromStr>::Err: std::error::Error + Sync + Send + 'static,
    {
        todo!()
    }

    fn validate_color(color: &str) -> Result<()> {
        todo!()
    }

    fn validate_message_format(format: &str) -> Result<()> {
        todo!()
    }

    fn add_color_arg(&mut self, value: &str) {
        todo!()
    }
}

impl ParsedSubcommand {
    fn parse(args: impl Iterator<Item = String>, current_exe: String) -> Result<Self> {
        todo!()
    }
}
