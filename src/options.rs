use crate::cargo_toml::CargoToml;
use anyhow::{format_err, Context, Error, Result};
use const_format::concatcp;
use std::{env, iter::Peekable, path::Path, str::FromStr, time::Duration};

const PROGRAM_ARGS_DELIMITER: &str = "--";

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

// TODO: Default
pub struct Options {
    // TODO: getset?
    pub cargo_args: Vec<String>,
    pub terminal_supports_colors: bool,
    pub limit_messages: usize,
    pub time_limit_after_error: Duration,
    pub ascending_messages_order: bool,
    pub show_warnings_if_errors_exist: bool,
    pub show_dependencies_warnings: bool,
    pub open_in_external_application: String,
    pub open_in_external_application_on_warnings: bool,
    pub help: bool,
    pub version: bool,
    pub json_message_format: bool,
    pub short_message_format: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            cargo_args: Vec::new(),
            terminal_supports_colors: true,
            limit_messages: 0,
            time_limit_after_error: Duration::from_secs(1),
            ascending_messages_order: false,
            show_warnings_if_errors_exist: false,
            show_dependencies_warnings: false,
            open_in_external_application: "".to_string(),
            open_in_external_application_on_warnings: false,
            help: false,    // TODO: WTF
            version: false, // TODO: WTF
            json_message_format: false,
            short_message_format: false,
        }
    }
}

impl Options {
    pub fn from_args_and_os(workspace_root: &Path) -> Result<Self> {
        Self::from_vars_and_atty()?.process_args(&mut env::args().peekable(), workspace_root)
    }

    fn from_vars_and_atty() -> Result<Self> {
        let mut result = Self::default();
        result.terminal_supports_colors = atty::is(atty::Stream::Stderr);
        Self::parse_var("CARGO_MSG_LIMIT", &mut result.limit_messages)?;
        {
            // TODO
            let mut seconds = result.time_limit_after_error.as_secs();
            Self::parse_var("CARGO_TIME_LIMIT", &mut seconds)?;
            result.time_limit_after_error = Duration::from_secs(seconds);
        }
        Self::parse_var("CARGO_ASC", &mut result.ascending_messages_order)?;
        Self::parse_var(
            "CARGO_FORCE_WARN",
            &mut result.show_warnings_if_errors_exist,
        )?;
        Self::parse_var("CARGO_DEPS_WARN", &mut result.show_dependencies_warnings)?;
        Self::parse_var("CARGO_OPEN", &mut result.open_in_external_application)?;
        Self::parse_var(
            "CARGO_OPEN_WARN",
            &mut result.open_in_external_application_on_warnings,
        )?;
        Ok(result)
    }

    fn process_args(
        mut self,
        args: impl Iterator<Item = String>,
        workspace_root: &Path, // TODO: should not be here?
    ) -> Result<Self> {
        let mut passed_args = args.skip(1).peekable();

        let cargo_command: String = passed_args
            .next()
            .ok_or_else(|| format_err!("cargo command not found"))?;

        let (first_letter, cargo_command) = cargo_command // TODO: either don't crash or crash everywhere
            .split_at(1);
        assert_eq!(first_letter, "l");
        self.cargo_args.push(cargo_command.to_owned());

        let mut program_args_started =
            if let Some(first_argument_after_cargo_command) = passed_args.peek() {
                // https://github.com/alopatindev/cargo-limit/issues/6
                !first_argument_after_cargo_command.starts_with('-')
            } else {
                false
            };

        let mut color = COLOR_AUTO.to_owned();
        self.process_main_args(&mut color, &mut passed_args, &mut program_args_started)?;
        self.process_color_args(
            color,
            passed_args,
            cargo_command,
            program_args_started,
            workspace_root,
        )?;

        Ok(self)
    }

    fn process_main_args(
        &mut self,
        color: &mut String,
        passed_args: &mut impl Iterator<Item = String>,
        program_args_started: &mut bool,
    ) -> Result<()> {
        if *program_args_started {
            return Ok(());
        }
        while let Some(arg) = passed_args.next() {
            if arg == "-h" || arg == "--help" {
                self.help = true;
                self.cargo_args.push(arg);
            } else if arg == "-v" || arg == "--version" {
                self.version = true;
                self.cargo_args.push(arg);
            } else if arg == COLOR[0..COLOR.len() - 1] {
                // TODO: don't use []
                *color = passed_args.next().context(
                    "the argument '--color <WHEN>' requires a value but none was supplied",
                )?;
                Self::validate_color(&color)?;
            } else if let Some(color_value) = arg.strip_prefix(COLOR) {
                *color = color_value.to_owned();
                Self::validate_color(&color)?;
            } else if arg == MESSAGE_FORMAT[0..MESSAGE_FORMAT.len() - 1] {
                let format = passed_args.next().context(
                    "the argument '--message-format <FMT>' requires a value but none was supplied",
                )?;
                Self::validate_message_format(&format)?;
                if format.starts_with(JSON_FORMAT) {
                    self.json_message_format = true;
                    self.cargo_args.push(arg);
                    self.cargo_args.push(format);
                } else if format == SHORT_FORMAT {
                    self.short_message_format = true;
                }
            } else if let Some(format) = arg.strip_prefix(MESSAGE_FORMAT) {
                Self::validate_message_format(&format)?;
                if format.starts_with(JSON_FORMAT) {
                    self.json_message_format = true;
                    self.cargo_args.push(arg);
                } else if format == SHORT_FORMAT {
                    self.short_message_format = true;
                }
            } else if arg == PROGRAM_ARGS_DELIMITER {
                *program_args_started = true;
                break;
            } else {
                self.cargo_args.push(arg);
            }
        }

        Ok(())
    }

    fn process_color_args(
        &mut self,
        color: String,
        passed_args: impl Iterator<Item = String>,
        cargo_command: &str,
        program_args_started: bool,
        workspace_root: &Path,
    ) -> Result<()> {
        if self.short_message_format {
            self.cargo_args.push(MESSAGE_FORMAT_JSON_SHORT.to_owned());
        } else if !self.json_message_format {
            let message_format_arg = if color == COLOR_AUTO {
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
            };
            self.cargo_args.push(message_format_arg.to_owned());
        }

        let mut program_color_is_set = false;
        if program_args_started {
            self.cargo_args.push(PROGRAM_ARGS_DELIMITER.to_owned());
            for arg in passed_args {
                if arg == COLOR[0..COLOR.len() - 1] || arg.starts_with(COLOR) {
                    program_color_is_set = true;
                }
                self.cargo_args.push(arg);
            }
        }

        if !program_args_started {
            self.cargo_args.push(PROGRAM_ARGS_DELIMITER.to_owned());
        }

        let is_test = cargo_command == "test";
        let is_bench = cargo_command == "bench";
        let command_supports_color_arg = is_test || is_bench;
        if command_supports_color_arg && !program_color_is_set && self.terminal_supports_colors {
            let cargo_toml = CargoToml::parse(workspace_root)?;
            let all_items_have_harness = if is_test {
                cargo_toml.all_tests_have_harness()
            } else if is_bench {
                cargo_toml.all_benchmarks_have_harness()
            } else {
                unreachable!()
            };
            if all_items_have_harness {
                // Workaround for programs that can't understand that terminal supports colors.
                // To fix that properly we need to run programs in pty.
                self.add_color_arg(COLOR_ALWAYS);
            }
        }
        Ok(())
    }

    fn parse_var<T: FromStr>(key: &str, value: &mut T) -> Result<()>
    where
        <T as FromStr>::Err: std::error::Error + Sync + Send + 'static,
    {
        if let Ok(new_value) = env::var(key) {
            *value = new_value
                .parse()
                .context(format!("invalid {} value", key))?; // TODO: with_context
        }
        Ok(())
    }

    fn validate_color(color: &str) -> Result<()> {
        if !VALID_COLORS.contains(&color) {
            return Err(format_err!(
                "argument for {} must be {} (was {})",
                &COLOR[0..COLOR.len() - 1],
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
                &MESSAGE_FORMAT[0..MESSAGE_FORMAT.len() - 1],
                VALID_MESSAGE_FORMATS.join(", "),
                format,
            ));
        }
        Ok(())
    }

    fn add_color_arg(&mut self, value: &str) {
        self.cargo_args.push(format!("{}{}", COLOR, value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_args() -> Result<()> {
        let cargo_bin = "/usr/bin/cargo";

        assert_cargo_args(
            vec![cargo_bin, "ltest"],
            vec![
                "test",
                "--message-format=json-diagnostic-rendered-ansi",
                "--",
                "--color=always",
            ],
        )?;

        assert_cargo_args(
            vec![
                cargo_bin,
                "lrun",
                "--cargo-argument",
                "other-cargo-argument",
            ],
            vec![
                "run",
                "--cargo-argument",
                "other-cargo-argument",
                "--message-format=json-diagnostic-rendered-ansi",
                "--",
                // "--color=always", // TODO?
            ],
        )?;

        assert_cargo_args(
            vec![cargo_bin, "lrun", "program-argument"], // https://github.com/alopatindev/cargo-limit/issues/6
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "--",
                "program-argument",
            ],
        )?;

        assert_cargo_args(
            vec![cargo_bin, "lrun", "--", "program-argument"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "--",
                "program-argument",
            ],
        )?;

        // TODO: colors (both for app and run), other options, harness
        /*assert_cargo_args(
            vec![cargo_bin, "lrun", "--color=always"],
            vec![
                "run",
                "--color=always",
                "--message-format=json-diagnostic-rendered-ansi",
                "--",
            ],
        )?;

        assert_cargo_args(
            vec!["lrun", "--color=never"],
            vec![
                "run",
                "--color=never",
                "--message-format=json-diagnostic-rendered-ansi",
                "--",
            ],
        )?;*/

        //assert_cargo_args(vec!["run", "--color", "never"], vec!["run", "--color", "never"])?; // TODO

        assert_cargo_args(
            vec![cargo_bin, "lrun", "--", "--color=always"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "--",
                "--color=always",
            ],
        )?;

        //assert_cargo_args(vec![cargo_bin, "--version"], vec!["--version"])?; // TODO: WTF

        //assert_cargo_args(vec![cargo_bin, "--help"], vec!["--help"])?; // TODO: WTF

        // TODO: message-format
        Ok(())
    }

    // TODO: test "lrun --message-format=..." args as well

    fn assert_cargo_args(input: Vec<&str>, expected_cargo_args: Vec<&str>) -> Result<()> {
        let options = Options::process_args(
            Options::default(),
            input.into_iter().map(|i| i.to_string()),
            Path::new("tests/stubs/minimal"),
        )?;
        assert_eq!(options.cargo_args, expected_cargo_args);
        Ok(())
    }
}
