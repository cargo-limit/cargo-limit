use crate::cargo_toml::CargoToml;
use anyhow::{format_err, Context, Result};
use const_format::concatcp;
use itertools::Either;
use std::{env, iter, path::Path, str::FromStr, time::Duration};

const EXECUTABLE_PREFIX: &str = "cargo-l";

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

impl Default for Options {
    fn default() -> Self {
        Self {
            cargo_args: Vec::new(),
            args_after_app_args_delimiter: Vec::new(),
            terminal_supports_colors: true,
            limit_messages: 0,
            time_limit_after_error: Duration::from_secs(1),
            ascending_messages_order: false,
            show_warnings_if_errors_exist: false,
            show_dependencies_warnings: false,
            open_in_external_app: "".to_owned(),
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

    pub fn from_os_env(workspace_root: &Path) -> Result<Self> {
        Self::from_vars_and_atty()?.process_args(env::args(), workspace_root)
    }

    fn from_vars_and_atty() -> Result<Self> {
        let mut result = Self::default();
        result.detect_terminal_color_support();

        {
            let mut seconds = result.time_limit_after_error.as_secs();
            Self::parse_var("CARGO_TIME_LIMIT", &mut seconds)?;
            result.time_limit_after_error = Duration::from_secs(seconds);
        }

        Self::parse_var("CARGO_MSG_LIMIT", &mut result.limit_messages)?;
        Self::parse_var("CARGO_ASC", &mut result.ascending_messages_order)?;
        Self::parse_var(
            "CARGO_FORCE_WARN",
            &mut result.show_warnings_if_errors_exist,
        )?;
        Self::parse_var("CARGO_DEPS_WARN", &mut result.show_dependencies_warnings)?;
        Self::parse_var("CARGO_OPEN", &mut result.open_in_external_app)?;
        Self::parse_var(
            "CARGO_OPEN_WARN",
            &mut result.open_in_external_app_on_warnings,
        )?;

        Ok(result)
    }

    fn detect_terminal_color_support(&mut self) {
        self.terminal_supports_colors = atty::is(atty::Stream::Stderr);
    }

    fn process_args(
        mut self,
        mut args: impl Iterator<Item = String>,
        workspace_root: &Path,
    ) -> Result<Self> {
        let first_arg = args
            .next()
            .ok_or_else(|| format_err!("invalid arguments"))?;

        let executable = if first_arg.starts_with(EXECUTABLE_PREFIX) {
            first_arg
        } else {
            let executable = std::path::PathBuf::from(first_arg)
                .into_iter()
                .last()
                .and_then(|i| i.to_str().map(|j| j.to_owned()))
                .ok_or_else(|| format_err!("invalid arguments"))?;
            let _ = args.next();
            executable
        };

        let (_prefix, cargo_subcommand) = try_split_at(&executable, EXECUTABLE_PREFIX.len())?;

        self.cargo_args.push(cargo_subcommand.to_owned());

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

        self.process_custom_runners(cargo_subcommand, app_color_is_set, workspace_root)?;

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
        cargo_subcommand: &str,
        app_color_is_set: bool,
        workspace_root: &Path,
    ) -> Result<()> {
        let is_test = cargo_subcommand == "test";
        let is_bench = cargo_subcommand == "bench";
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
            if arg == COLOR[0..COLOR.len() - 1] || arg.starts_with(COLOR) {
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
        self.args_after_app_args_delimiter
            .push(format!("{}{}", COLOR, value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    const STUB_MINIMAL: &str = "minimal";
    const STUB_CUSTOM_TEST_RUNNER: &str = "custom_test_runner";
    const STUB_CUSTOM_BENCH_RUNNER: &str = "custom_bench_runner";

    #[test]
    fn smoke() -> Result<()> {
        assert_cargo_args(
            vec!["cargo-lrun", "--", "app-argument"],
            vec!["run", "--message-format=json-diagnostic-rendered-ansi"],
            vec!["app-argument"],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec!["cargo-lrun", "-vvv", "--", "-c", "app-config.yml"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "-vvv",
            ],
            vec!["-c", "app-config.yml"],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec!["cargo-lrun", "-p=app", "--", "-c", "app-config.yml"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "-p=app",
            ],
            vec!["-c", "app-config.yml"],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec!["cargo-lrun", "-p", "app", "--", "-c", "app-config.yml"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "-p",
                "app",
            ],
            vec!["-c", "app-config.yml"],
            STUB_MINIMAL,
        )?;

        assert_options(
            vec!["cargo-lclippy", "--help"],
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
            vec!["cargo-lclippy", "--version"],
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
            vec!["cargo-lclippy", "-V"],
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
            vec!["cargo-ltest", "--", "--help"],
            vec!["test", "--message-format=json-diagnostic-rendered-ansi"],
            vec!["--help", "--color=always"],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec!["cargo-lrun", "--verbose"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "--verbose",
            ],
            vec![],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec!["cargo-lrun", "-v"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "-v",
            ],
            vec![],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec!["cargo-lrun", "-vv"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "-vv",
            ],
            vec![],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec!["cargo-lrun", "-v", "-v"],
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
            vec!["cargo-lrun", "-v", "-v", "app-arg"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "-v",
                "-v",
                "app-arg",
            ],
            vec![],
            STUB_MINIMAL,
        )?;

        assert_options(
            vec![
                "cargo-lrun",
                "-v",
                "--message-format=short",
                "-v",
                "app-arg",
            ],
            vec![
                "run",
                "--message-format=json-diagnostic-short",
                "-v",
                "-v",
                "app-arg",
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
                "cargo-lrun",
                "-v",
                "-v",
                "--message-format=human",
                "app-arg",
            ],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "-v",
                "-v",
                "app-arg",
            ],
            vec![],
            STUB_MINIMAL,
        )?;

        assert_options(
            vec!["cargo-lrun", "-v", "-v", "--message-format=json", "app-arg"],
            vec!["run", "--message-format=json", "-v", "-v", "app-arg"],
            vec![],
            Options {
                json_message_format: true,
                ..Options::default()
            },
            STUB_MINIMAL,
        )?;

        Ok(())
    }

    // TODO: naming
    // FIXME
    // TODO: remove unnecessary
    #[test]
    fn wat() -> Result<()> {
        assert_cargo_args(
            vec!["carog-lrun", "app-arg"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "app-arg",
            ],
            vec![],
            STUB_MINIMAL,
        )?;
        assert_cargo_args(
            vec!["carog-lrun", "-v", "-v", "-v", "app-arg"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "-v",
                "-v",
                "-v",
                "app-arg",
            ],
            vec![],
            STUB_MINIMAL,
        )?;
        assert_cargo_args(
            vec!["carog-lrun", "-v", "-v", "app-arg"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "-v",
                "-v",
                "app-arg",
            ],
            vec![],
            STUB_MINIMAL,
        )?;
        Ok(())
    }

    #[test]
    fn test_with_message_format() -> Result<()> {
        assert_options(
            vec!["cargo-ltest", "--message-format=json"],
            vec!["test", "--message-format=json"],
            vec!["--color=always"],
            Options {
                json_message_format: true,
                ..Options::default()
            },
            STUB_MINIMAL,
        )?;

        assert_options(
            vec!["cargo-ltest", "--message-format=short"],
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
            vec!["cargo-lrun", "--color=always"],
            vec!["run", "--message-format=json-diagnostic-rendered-ansi"],
            vec![],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec!["cargo-lrun", "--color=never"],
            vec!["run", "--message-format=json"],
            vec![],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec!["cargo-lrun", "--", "--color=always"],
            vec!["run", "--message-format=json-diagnostic-rendered-ansi"],
            vec!["--color=always"],
            STUB_MINIMAL,
        )?;

        Ok(())
    }

    #[test]
    fn colored_testing_and_compiling_1() -> Result<()> {
        assert_cargo_args(
            vec!["cargo-ltest"],
            vec!["test", "--message-format=json-diagnostic-rendered-ansi"],
            vec!["--color=always"],
            STUB_MINIMAL,
        )?;
        Ok(())
    }

    #[test]
    fn colored_testing_and_compiling_2() -> Result<()> {
        assert_cargo_args(
            vec!["cargo-ltest", "--color=always"],
            vec!["test", "--message-format=json-diagnostic-rendered-ansi"],
            vec!["--color=always"],
            STUB_MINIMAL,
        )?;
        Ok(())
    }

    #[test]
    fn colored_testing_and_compiling_3() -> Result<()> {
        assert_cargo_args(
            vec!["cargo-ltest", "--", "--color=always"],
            vec!["test", "--message-format=json-diagnostic-rendered-ansi"],
            vec!["--color=always"],
            STUB_MINIMAL,
        )?;

        Ok(())
    }

    #[test]
    fn colored_testing_and_monochrome_compiling_1() -> Result<()> {
        assert_cargo_args(
            vec!["cargo-ltest", "--color=never"],
            vec!["test", "--message-format=json"],
            vec!["--color=always"],
            STUB_MINIMAL,
        )?;
        Ok(())
    }

    #[test]
    fn monochrome_testing_and_colored_compiling_1() -> Result<()> {
        assert_cargo_args(
            vec!["cargo-ltest", "--", "--color=never"],
            vec!["test", "--message-format=json-diagnostic-rendered-ansi"],
            vec!["--color=never"],
            STUB_MINIMAL,
        )?;
        Ok(())
    }

    #[test]
    fn custom_runners_should_not_have_color_args() -> Result<()> {
        assert_cargo_args(
            vec!["cargo-ltest"],
            vec!["test", "--message-format=json-diagnostic-rendered-ansi"],
            vec![],
            STUB_CUSTOM_TEST_RUNNER,
        )?;

        assert_cargo_args(
            vec!["cargo-ltest", "--", "--runner-arg"],
            vec!["test", "--message-format=json-diagnostic-rendered-ansi"],
            vec!["--runner-arg"],
            STUB_CUSTOM_TEST_RUNNER,
        )?;

        assert_cargo_args(
            vec!["cargo-lbench"],
            vec!["bench", "--message-format=json-diagnostic-rendered-ansi"],
            vec![],
            STUB_CUSTOM_BENCH_RUNNER,
        )?;

        assert_options(
            vec!["cargo-lbench", "--help"],
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
            vec!["cargo-lrun", "--", "--"],
            vec!["run", "--message-format=json-diagnostic-rendered-ansi"],
            vec!["--"],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec!["cargo-lrun", "--", "--", "1"],
            vec!["run", "--message-format=json-diagnostic-rendered-ansi"],
            vec!["--", "1"],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec!["cargo-lrun", "--", "-", "1", "2", "3"],
            vec!["run", "--message-format=json-diagnostic-rendered-ansi"],
            vec!["-", "1", "2", "3"],
            STUB_MINIMAL,
        )?;

        Ok(())
    }

    #[test]
    fn app_args_without_two_dashes_splitter() -> Result<()> {
        assert_cargo_args(
            vec!["cargo-lrun", "app-argument"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "app-argument",
            ],
            vec![],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec!["cargo-lrun", "-"],
            vec!["run", "--message-format=json-diagnostic-rendered-ansi", "-"],
            vec![],
            STUB_MINIMAL,
        )?;

        assert_cargo_args(
            vec!["cargo-lrun", "1", "2", "3"],
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
            vec!["cargo-lrun", "-", "1", "2", "3"],
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
            vec!["cargo-lrun", "--verbose", "app-argument"],
            vec![
                "run",
                "--message-format=json-diagnostic-rendered-ansi",
                "--verbose",
                "app-argument",
            ],
            vec![],
            STUB_MINIMAL,
        )?;

        Ok(())
    }

    fn assert_cargo_args(
        input: Vec<&str>,
        expected_cargo_args: Vec<&str>,
        expected_args_after_app_args_delimiter: Vec<&str>,
        stub: &str,
    ) -> Result<()> {
        assert_options(
            input,
            expected_cargo_args,
            expected_args_after_app_args_delimiter,
            Default::default(),
            stub,
        )
    }

    fn assert_options(
        input: Vec<&str>,
        expected_cargo_args: Vec<&str>,
        expected_args_after_app_args_delimiter: Vec<&str>,
        expected_options: Options,
        stub: &str,
    ) -> Result<()> {
        fn to_string<'item>(
            iter: impl IntoIterator<Item = &'item str> + 'item,
        ) -> impl Iterator<Item = String> + 'item {
            iter.into_iter().map(|i| i.to_owned())
        }

        let options = Options::process_args(
            Options::default(),
            to_string(input),
            &Path::new("tests/stubs").join(Path::new(stub)),
        )?;

        let expected = Options {
            cargo_args: to_string(expected_cargo_args).collect(),
            args_after_app_args_delimiter: to_string(expected_args_after_app_args_delimiter)
                .collect(),
            ..expected_options
        };

        assert_eq!(options, expected);
        Ok(())
    }
}

fn try_split_at(input: &str, index: usize) -> Result<(&str, &str)> {
    if index > input.len() {
        Err(format_err!("cannot split '{}' at {}", input, index))
    } else {
        Ok(input.split_at(index))
    }
}
