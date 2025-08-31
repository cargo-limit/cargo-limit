use crate::{cargo_toml::CargoToml, process::CARGO_EXECUTABLE};
use anyhow::{format_err, Context, Result};
use const_format::concatcp;
use itertools::Either;
use std::{env, io, io::IsTerminal, iter, path::Path, str::FromStr, time::Duration};

const EXECUTABLE_PREFIX: &str = concatcp!(CARGO_EXECUTABLE, "-l");

const APP_ARGS_DELIMITER: &str = "--";

const MESSAGE_FORMAT: &str = "--message-format=";
const MESSAGE_FORMAT_JSON: &str = "";
const MESSAGE_FORMAT_JSON_WITH_COLORS: &str = "";
const MESSAGE_FORMAT_JSON_SHORT: &str = "";

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
            Self::parse_var("CARGO_TIME_LIMIT", &mut seconds)?;

            let duration = Duration::from_secs(seconds);
            result.time_limit_after_error = if duration > Duration::from_secs(0) {
                Some(duration)
            } else {
                None
            };
        }

        Self::parse_var("CARGO_MSG_LIMIT", &mut result.limit_messages)?;
        Self::parse_var("CARGO_ASC", &mut result.ascending_messages_order)?;
        Self::parse_var(
            "CARGO_FORCE_WARN",
            &mut result.show_warnings_if_errors_exist,
        )?;
        Self::parse_var("CARGO_DEPS_WARN", &mut result.show_dependencies_warnings)?;
        Self::parse_var("CARGO_EDITOR", &mut result.open_in_external_app)?;

        Ok(result)
    }

    fn detect_terminal_color_support(&mut self) {
        self.terminal_supports_colors = io::stderr().is_terminal()
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
        while let Some(arg) = passed_args.next() {
            if arg == "-h" || arg == "--help" {
                self.help = true;
                args_before_app_args_delimiter.push(arg);
            } else if arg == "-V" || arg == "--version" {
                self.version = true;
                args_before_app_args_delimiter.push(arg);
            } else if arg == COLOR[..COLOR.len() - 1] {
                *color = passed_args.next().context(
                    "the argument '--color <WHEN>' requires a value but none was supplied",
                )?;
                Self::validate_color(color)?;
            } else if let Some(color_value) = arg.strip_prefix(COLOR) {
                *color = color_value.to_owned();
                Self::validate_color(color)?;
            } else if arg == MESSAGE_FORMAT[..MESSAGE_FORMAT.len() - 1] {
                let format = passed_args.next().context(
                    "the argument '--message-format <FMT>' requires a value but none was supplied",
                )?;
                Self::validate_message_format(&format)?;
                if format.starts_with(JSON_FORMAT) {
                    self.json_message_format = true;
                } else if format == SHORT_FORMAT {
                    self.short_message_format = true;
                }
            } else if let Some(format) = arg.strip_prefix(MESSAGE_FORMAT) {
                Self::validate_message_format(format)?;
                if format.starts_with(JSON_FORMAT) {
                    self.json_message_format = true;
                } else if format == SHORT_FORMAT {
                    self.short_message_format = true;
                }
            } else if arg == APP_ARGS_DELIMITER {
                *app_args_started = true;
                break;
            } else {
                args_before_app_args_delimiter.push(arg);
            }
        }

        Ok(())
    }

    fn message_format(&self, color: String) -> &str {
        if self.short_message_format {
            MESSAGE_FORMAT_JSON_SHORT
        } else if self.json_message_format {
            MESSAGE_FORMAT_JSON
        } else if color == COLOR_AUTO {
            if self.terminal_supports_colors {
                MESSAGE_FORMAT_JSON_WITH_COLORS
            } else {
                MESSAGE_FORMAT_JSON
            }
        } else if color == COLOR_ALWAYS {
            MESSAGE_FORMAT_JSON_WITH_COLORS
        } else if color == COLOR_NEVER {
            MESSAGE_FORMAT_JSON
        } else {
            unreachable!()
        }
    }

    fn process_custom_runners(
        &mut self,
        subcommand: String,
        app_color_is_set: bool,
        workspace_root: &Path,
    ) -> Result<()> {
        let is_test = subcommand == "test";
        let is_bench = subcommand == "bench";
        let command_supports_color_arg = is_test || is_bench;
        if command_supports_color_arg && !app_color_is_set && self.terminal_supports_colors {
            let cargo_toml = CargoToml::parse(workspace_root)?;
            let all_items_have_harness = if is_test {
                cargo_toml.all_tests_have_harness()
            } else if is_bench {
                cargo_toml.all_benchmarks_have_harness()
            } else {
                unreachable!()
            };
            if all_items_have_harness {
                // Workaround for apps that can't understand that terminal supports colors.
                // To fix that properly we need to run apps in pty.
                // https://github.com/alopatindev/cargo-limit/issues/4#issuecomment-833692334
                self.add_color_arg(COLOR_ALWAYS);
            }
        }
        Ok(())
    }

    fn process_args_after_app_args_delimiter(
        &mut self,
        passed_args: impl Iterator<Item = String>,
        app_color_is_set: &mut bool,
    ) {
        for arg in passed_args {
            if arg == COLOR[..COLOR.len() - 1] || arg.starts_with(COLOR) {
                *app_color_is_set = true;
            }
            self.args_after_app_args_delimiter.push(arg);
        }
    }

    fn parse_var<T: FromStr>(key: &str, value: &mut T) -> Result<()>
    where
        <T as FromStr>::Err: std::error::Error + Sync + Send + 'static,
    {
        if let Ok(new_value) = env::var(key) {
            *value = new_value
                .parse()
                .with_context(|| format!("invalid {} value", key))?;
        }
        Ok(())
    }

    fn validate_color(color: &str) -> Result<()> {
        if !VALID_COLORS.contains(&color) {
            return Err(format_err!(
                "argument for {} must be {} (was {})",
                &COLOR[..COLOR.len() - 1],
                VALID_COLORS.join(", "),
                color,
            ));
        }
        Ok(())
    }

    fn validate_message_format(format: &str) -> Result<()> {
        if !VALID_MESSAGE_FORMATS.contains(&format) {
            return Err(format_err!(
                "argument for {} must be {} (was {})",
                &MESSAGE_FORMAT[..MESSAGE_FORMAT.len() - 1],
                VALID_MESSAGE_FORMATS.join(", "),
                format,
            ));
        }
        Ok(())
    }

    fn add_color_arg(&mut self, value: &str) {
        self.args_after_app_args_delimiter
            .push(format!("{}{}", COLOR, value));
    }
}

impl ParsedSubcommand {
    fn parse(args: impl Iterator<Item = String>, current_exe: String) -> Result<Self> {
        let current_exe = current_exe.to_lowercase();
        let (_, subcommand) = current_exe
            .split_once(EXECUTABLE_PREFIX)
            .context("invalid arguments")?;
        let (open_in_external_app_on_warnings, subcommand) = if subcommand.starts_with('l') {
            let (_, subcommand) = subcommand.split_once('l').context("invalid arguments")?;
            (true, subcommand)
        } else {
            (false, subcommand)
        };

        let mut peekable_args = args.peekable();
        loop {
            let arg = peekable_args.peek();
            let executable = arg
                .and_then(|arg| Path::new(arg).file_stem())
                .map(|i| i.to_string_lossy());
            if let Some(executable) = executable {
                if executable == CARGO_EXECUTABLE
                    || executable == current_exe
                    || executable == format!("l{}", subcommand)
                    || executable == format!("ll{}", subcommand)
                {
                    let _ = peekable_args.next();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(Self {
            subcommand: subcommand.to_owned(),
            open_in_external_app_on_warnings,
            remaining_args: peekable_args.collect(),
        })
    }
}
