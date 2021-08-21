use crate::cargo_toml::CargoToml;
use anyhow::{format_err, Context, Result};
use const_format::concatcp;
use itertools::Either;
use std::{env, iter, path::Path, str::FromStr, time::Duration};

const PROGRAM_ARGS_DELIMITER: &str = "--";

const MESSAGE_FORMAT: &str = "--message-format=";
const MESSAGE_FORMAT_JSON: &str = concatcp!(MESSAGE_FORMAT, JSON_FORMAT);
const MESSAGE_FORMAT_JSON_WITH_COLORS: &str = concatcp!(MESSAGE_FORMAT, JSON_FORMAT_WITH_COLORS);
const MESSAGE_FORMAT_JSON_SHORT: &str = concatcp!(MESSAGE_FORMAT, JSON_FORMAT_SHORT);

const JSON_FORMAT: &str = "json";
const JSON_FORMAT_WITH_COLORS: &str = "json-diagnostic-rendered-ansi";
const JSON_FORMAT_SHORT: &str = "json-diagnostic-short";
const SHORT_FORMAT: &str = "short";
const HUMAN_FORMAT: &str = "human"; // TODO: test it
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
    // TODO: getset?
    pub cargo_args: Vec<String>,
    pub args_after_program_args_delimiter: Vec<String>,
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
            args_after_program_args_delimiter: Vec::new(),
            terminal_supports_colors: true,
            limit_messages: 0,
            time_limit_after_error: Duration::from_secs(1),
            ascending_messages_order: false,
            show_warnings_if_errors_exist: false,
            show_dependencies_warnings: false,
            open_in_external_application: "".to_string(), // TODO: to_owned => to_string everywhere? or the opposite
            open_in_external_application_on_warnings: false,
            help: false,
            version: false,
            json_message_format: false,
            short_message_format: false,
        }
    }
}

impl Options {
    pub fn all_args(&self) -> impl Iterator<Item = String> {
        let delimiter = if self.args_after_program_args_delimiter.is_empty() {
            Either::Left(iter::empty())
        } else {
            Either::Right(iter::once(PROGRAM_ARGS_DELIMITER.to_string()))
        };
        self.cargo_args
            .clone()
            .into_iter()
            .chain(delimiter)
            .chain(self.args_after_program_args_delimiter.clone())
    }

    pub fn from_args_and_os(workspace_root: &Path) -> Result<Self> {
        Self::from_vars_and_atty()?.process_args(&mut env::args(), workspace_root)
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
        let mut passed_args = args.skip(1);
        let cargo_command = passed_args
            .next()
            .ok_or_else(|| format_err!("cargo command not found"))?;
        let (first_letter, cargo_command) = cargo_command // TODO: either don't crash or crash everywhere
            .split_at(1);
        assert_eq!(first_letter, "l");
        self.cargo_args.push(cargo_command.to_owned()); // TODO: which means it's not really args
                                                        // TODO: cargo_command => cargo_subcommand

        let mut color = COLOR_AUTO.to_owned();
        let passed_args = passed_args.collect::<Vec<_>>();
        self.parse_options(passed_args.clone().into_iter(), &mut color)?;
        self.cargo_args.push(self.message_format(color).to_owned());

        // TODO: program => app
        let mut program_args_started = false;
        let mut passed_args = passed_args.into_iter();
        self.consume_remaining_args(&mut passed_args, &mut program_args_started)?;

        let mut program_color_is_set = false;
        if program_args_started {
            self.process_args_after_program_args_delimiter(passed_args, &mut program_color_is_set);
        }

        self.process_color_and_program_args(cargo_command, program_color_is_set, workspace_root)?;

        Ok(self)
    }

    fn parse_options(
        &mut self,
        mut passed_args: impl Iterator<Item = String>,
        color: &mut String,
    ) -> Result<()> {
        while let Some(arg) = passed_args.next() {
            // TODO: extract consts
            if arg == "-h" || arg == "--help" {
                self.help = true;
            } else if arg == "-V" || arg == "--version" {
                self.version = true;
            } else if arg == COLOR[0..COLOR.len() - 1] {
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
                } else if format == SHORT_FORMAT {
                    self.short_message_format = true;
                }
            } else if let Some(format) = arg.strip_prefix(MESSAGE_FORMAT) {
                Self::validate_message_format(&format)?;
                if format.starts_with(JSON_FORMAT) {
                    self.json_message_format = true;
                } else if format == SHORT_FORMAT {
                    self.short_message_format = true;
                }
            } else if arg == PROGRAM_ARGS_DELIMITER {
                break;
            }
        }

        Ok(())
    }

    fn message_format(&self, color: String) -> &str {
        if self.short_message_format {
            MESSAGE_FORMAT_JSON_SHORT
        } else if self.json_message_format {
            MESSAGE_FORMAT_JSON
        } else {
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
            message_format_arg
        }
    }

    fn consume_remaining_args(
        &mut self,
        passed_args: &mut impl Iterator<Item = String>,
        program_args_started: &mut bool,
    ) -> Result<()> {
        while let Some(arg) = passed_args.next() {
            if arg == "-h" || arg == "--help" {
                self.cargo_args.push(arg);
            } else if arg == "-V" || arg == "--version" {
                self.cargo_args.push(arg);
            } else if arg == COLOR[0..COLOR.len() - 1] { // TODO: cleanup
            } else if let Some(_) = arg.strip_prefix(COLOR) {
            } else if arg == MESSAGE_FORMAT[0..MESSAGE_FORMAT.len() - 1] {
            } else if let Some(_) = arg.strip_prefix(MESSAGE_FORMAT) {
            } else if arg == PROGRAM_ARGS_DELIMITER {
                *program_args_started = true;
                break;
            } else {
                self.cargo_args.push(arg); // TODO: should we even do this here?
            }
        }
        Ok(())
    }

    // TODO: naming
    fn process_color_and_program_args(
        &mut self,
        cargo_command: &str,
        program_color_is_set: bool, // TODO: too many args, flags are evil here
        workspace_root: &Path,
    ) -> Result<()> {
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
                // https://github.com/alopatindev/cargo-limit/issues/4#issuecomment-833692334
                self.add_color_arg(COLOR_ALWAYS);
            }
        }
        Ok(())
    }

    fn process_args_after_program_args_delimiter(
        &mut self,
        passed_args: impl Iterator<Item = String>,
        program_color_is_set: &mut bool,
    ) {
        for arg in passed_args {
            if arg == COLOR[0..COLOR.len() - 1] || arg.starts_with(COLOR) {
                *program_color_is_set = true;
            }
            self.args_after_program_args_delimiter.push(arg);
        }
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
        self.args_after_program_args_delimiter
            .push(format!("{}{}", COLOR, value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    const CARGO_BIN: &str = "/path/to/bin/cargo";
    const STUB_MINIMAL: &str = "minimal";
    const STUB_CUSTOM_TEST_RUNNER: &str = "custom_test_runner";
    const STUB_CUSTOM_BENCH_RUNNER: &str = "custom_bench_runner";

    #[test]
    fn smoke() -> Result<()> {
        assert_cargo_args(
            vec![CARGO_BIN, "lrun", "--", "program-argument"],
            vec!["run", "--message-format=json-diagnostic-rendered-ansi"],
            vec!["program-argument"],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec![CARGO_BIN, "lrun", "-vvv", "--", "-c", "program-config.yml"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "-vvv",
            ],
            vec!["-c", "program-config.yml"],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec![
                CARGO_BIN,
                "lrun",
                "-p=program",
                "--",
                "-c",
                "program-config.yml",
            ],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "-p=program",
            ],
            vec!["-c", "program-config.yml"],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec![
                CARGO_BIN,
                "lrun",
                "-p",
                "program",
                "--",
                "-c",
                "program-config.yml",
            ],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "-p",
                "program",
            ],
            vec!["-c", "program-config.yml"],
            STUB_MINIMAL,
        )?;

        assert_options(
            vec![CARGO_BIN, "lclippy", "--help"],
            vec![
                "clippy",
                "--message-format=json-diagnostic-rendered-ansi",
                "--help",
            ],
            vec![],
            Options {
                help: true,
                ..Options::default()
            },
            STUB_MINIMAL,
        )?;

        assert_options(
            vec![CARGO_BIN, "lclippy", "--version"],
            vec![
                "clippy",
                "--message-format=json-diagnostic-rendered-ansi",
                "--version",
            ],
            vec![],
            Options {
                version: true,
                ..Options::default()
            },
            STUB_MINIMAL,
        )?;

        assert_options(
            vec![CARGO_BIN, "lclippy", "-V"],
            vec![
                "clippy",
                "--message-format=json-diagnostic-rendered-ansi",
                "-V",
            ],
            vec![],
            Options {
                version: true,
                ..Options::default()
            },
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec![CARGO_BIN, "ltest", "--", "--help"],
            vec!["test", "--message-format=json-diagnostic-rendered-ansi"],
            vec!["--help", "--color=always"],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec![CARGO_BIN, "lrun", "--verbose"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "--verbose",
            ],
            vec![],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec![CARGO_BIN, "lrun", "-v"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "-v",
            ],
            vec![],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec![CARGO_BIN, "lrun", "-vv"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "-vv",
            ],
            vec![],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec![CARGO_BIN, "lrun", "-v", "-v"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "-v",
                "-v",
            ],
            vec![],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec![CARGO_BIN, "lrun", "-v", "-v", "program-arg"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "-v",
                "-v",
                "program-arg",
            ],
            vec![],
            STUB_MINIMAL,
        )?;

        assert_options(
            vec![
                CARGO_BIN,
                "lrun",
                "-v",
                "--message-format=short",
                "-v",
                "program-arg",
            ],
            vec![
                "run",
                "--message-format=json-diagnostic-short",
                "-v",
                "-v",
                "program-arg",
            ],
            vec![],
            Options {
                short_message_format: true,
                ..Options::default()
            },
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec![
                CARGO_BIN,
                "lrun",
                "-v",
                "-v",
                "--message-format=human",
                "program-arg",
            ],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "-v",
                "-v",
                "program-arg",
            ],
            vec![],
            STUB_MINIMAL,
        )?;

        assert_options(
            vec![
                CARGO_BIN,
                "lrun",
                "-v",
                "-v",
                "--message-format=json",
                "program-arg",
            ],
            vec!["run", "--message-format=json", "-v", "-v", "program-arg"],
            vec![],
            Options {
                json_message_format: true,
                ..Options::default()
            },
            STUB_MINIMAL,
        )?;

        Ok(())
    }

    #[test]
    fn wat() -> Result<()> {
        assert_cargo_args(
            vec![CARGO_BIN, "lrun", "-v", "-v", "program-arg"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "-v",
                "-v",
                "program-arg",
            ],
            vec![],
            STUB_MINIMAL,
        )?;
        Ok(())
    }

    #[test]
    fn test_with_message_format() -> Result<()> {
        assert_options(
            vec![CARGO_BIN, "ltest", "--message-format=json"],
            vec!["test", "--message-format=json"],
            vec!["--color=always"],
            Options {
                json_message_format: true,
                ..Options::default()
            },
            STUB_MINIMAL,
        )?;

        assert_options(
            vec![CARGO_BIN, "ltest", "--message-format=short"],
            vec!["test", "--message-format=json-diagnostic-short"],
            vec!["--color=always"],
            Options {
                short_message_format: true,
                ..Options::default()
            },
            STUB_MINIMAL,
        )?;

        Ok(())
    }

    #[test]
    fn run_with_color_args() -> Result<()> {
        assert_cargo_args(
            vec![CARGO_BIN, "lrun", "--color=always"],
            vec!["run", "--message-format=json-diagnostic-rendered-ansi"],
            vec![],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec![CARGO_BIN, "lrun", "--color=never"],
            vec!["run", "--message-format=json"],
            vec![],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec![CARGO_BIN, "lrun", "--", "--color=always"],
            vec!["run", "--message-format=json-diagnostic-rendered-ansi"],
            vec!["--color=always"],
            STUB_MINIMAL,
        )?;

        Ok(())
    }

    #[test]
    fn colored_testing_and_compiling_1() -> Result<()> {
        assert_cargo_args(
            vec![CARGO_BIN, "ltest"],
            vec!["test", "--message-format=json-diagnostic-rendered-ansi"],
            vec!["--color=always"],
            STUB_MINIMAL,
        )?;
        Ok(())
    }

    #[test]
    fn colored_testing_and_compiling_2() -> Result<()> {
        assert_cargo_args(
            vec![CARGO_BIN, "ltest", "--color=always"],
            vec!["test", "--message-format=json-diagnostic-rendered-ansi"],
            vec!["--color=always"],
            STUB_MINIMAL,
        )?;
        Ok(())
    }

    #[test]
    fn colored_testing_and_compiling_3() -> Result<()> {
        assert_cargo_args(
            vec![CARGO_BIN, "ltest", "--", "--color=always"],
            vec!["test", "--message-format=json-diagnostic-rendered-ansi"],
            vec!["--color=always"],
            STUB_MINIMAL,
        )?;

        Ok(())
    }

    #[test]
    fn colored_testing_and_monochrome_compiling_1() -> Result<()> {
        assert_cargo_args(
            vec![CARGO_BIN, "ltest", "--color=never"],
            vec!["test", "--message-format=json"],
            vec!["--color=always"],
            STUB_MINIMAL,
        )?;
        Ok(())
    }

    #[test]
    fn monochrome_testing_and_colored_compiling_1() -> Result<()> {
        assert_cargo_args(
            vec![CARGO_BIN, "ltest", "--", "--color=never"],
            vec!["test", "--message-format=json-diagnostic-rendered-ansi"],
            vec!["--color=never"],
            STUB_MINIMAL,
        )?;
        Ok(())
    }

    #[test]
    fn custom_runners_should_not_have_color_args() -> Result<()> {
        assert_cargo_args(
            vec![CARGO_BIN, "ltest"],
            vec!["test", "--message-format=json-diagnostic-rendered-ansi"],
            vec![],
            STUB_CUSTOM_TEST_RUNNER,
        )?;

        assert_cargo_args(
            vec![CARGO_BIN, "ltest", "--", "--runner-arg"],
            vec!["test", "--message-format=json-diagnostic-rendered-ansi"],
            vec!["--runner-arg"],
            STUB_CUSTOM_TEST_RUNNER,
        )?;

        assert_cargo_args(
            vec![CARGO_BIN, "lbench"],
            vec!["bench", "--message-format=json-diagnostic-rendered-ansi"],
            vec![],
            STUB_CUSTOM_BENCH_RUNNER,
        )?;

        assert_options(
            vec![CARGO_BIN, "lbench", "--help"],
            vec![
                "bench",
                "--message-format=json-diagnostic-rendered-ansi",
                "--help",
            ],
            vec![],
            Options {
                help: true,
                ..Options::default()
            },
            STUB_CUSTOM_BENCH_RUNNER,
        )?;

        Ok(())
    }

    #[test]
    fn double_two_dashes() -> Result<()> {
        assert_cargo_args(
            vec![CARGO_BIN, "lrun", "--", "--"],
            vec!["run", "--message-format=json-diagnostic-rendered-ansi"],
            vec!["--"],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec![CARGO_BIN, "lrun", "--", "--", "1"],
            vec!["run", "--message-format=json-diagnostic-rendered-ansi"],
            vec!["--", "1"],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec![CARGO_BIN, "lrun", "--", "-", "1", "2", "3"],
            vec!["run", "--message-format=json-diagnostic-rendered-ansi"],
            vec!["-", "1", "2", "3"],
            STUB_MINIMAL,
        )?;

        Ok(())
    }

    #[test]
    fn program_args_without_two_dashes_splitter() -> Result<()> {
        assert_cargo_args(
            vec![CARGO_BIN, "lrun", "program-argument"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "program-argument",
            ],
            vec![],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec![CARGO_BIN, "lrun", "-"],
            vec!["run", "--message-format=json-diagnostic-rendered-ansi", "-"],
            vec![],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec![CARGO_BIN, "lrun", "1", "2", "3"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "1",
                "2",
                "3",
            ],
            vec![],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec![CARGO_BIN, "lrun", "-", "1", "2", "3"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "-",
                "1",
                "2",
                "3",
            ],
            vec![],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec![CARGO_BIN, "lrun", "--verbose", "program-argument"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "--verbose",
                "program-argument",
            ],
            vec![],
            STUB_MINIMAL,
        )?;

        Ok(())
    }

    fn assert_cargo_args(
        input: Vec<&str>,
        expected_cargo_args: Vec<&str>,
        expected_args_after_program_args_delimiter: Vec<&str>,
        stub: &str,
    ) -> Result<()> {
        assert_options(
            input,
            expected_cargo_args,
            expected_args_after_program_args_delimiter,
            Default::default(),
            stub,
        )
    }

    fn assert_options(
        input: Vec<&str>,
        expected_cargo_args: Vec<&str>,
        expected_args_after_program_args_delimiter: Vec<&str>,
        expected_options: Options,
        stub: &str,
    ) -> Result<()> {
        // TODO: CARGO_BIN
        let options = Options::process_args(
            Options::default(),
            input.into_iter().map(|i| i.to_string()),
            &Path::new("tests/stubs").join(Path::new(stub)),
        )?;
        let expected = Options {
            cargo_args: expected_cargo_args
                .into_iter()
                .map(|i| i.to_string()) // TODO: extract
                .collect(),
            args_after_program_args_delimiter: expected_args_after_program_args_delimiter
                .into_iter()
                .map(|i| i.to_string())
                .collect(),
            ..expected_options
        };
        assert_eq!(options, expected);
        Ok(())
    }
}
