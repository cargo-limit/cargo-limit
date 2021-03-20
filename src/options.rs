use anyhow::{format_err, Context, Error, Result};
use const_format::concatcp;
use std::{env, str::FromStr, time::Duration};

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

pub struct Options {
    pub cargo_args: Vec<String>,
    pub limit_messages: usize,
    pub time_limit_after_error: Duration,
    pub ascending_messages_order: bool,
    pub show_warnings_if_errors_exist: bool,
    pub show_dependencies_warnings: bool,
    pub help: bool,
    pub json_message_format: bool,
    pub short_message_format: bool,
}

impl Options {
    pub fn from_args_and_vars(cargo_command: &str) -> Result<Self> {
        let mut passed_args = env::args().skip(2);
        let mut result = Self {
            cargo_args: Vec::new(),
            limit_messages: Self::parse_var("CARGO_MSG_LIMIT", "0")?,
            time_limit_after_error: Duration::from_secs(Self::parse_var("CARGO_TIME_LIMIT", "1")?),
            ascending_messages_order: Self::parse_var("CARGO_ASC", "false")?,
            show_warnings_if_errors_exist: Self::parse_var("CARGO_FORCE_WARN", "false")?,
            show_dependencies_warnings: Self::parse_var("CARGO_DEPS_WARN", "false")?,
            help: false,
            json_message_format: false,
            short_message_format: false,
        };
        let mut program_args_started = false;
        let mut color = COLOR_AUTO.to_owned();

        result.cargo_args.push(cargo_command.to_owned());
        result.process_main_args(&mut color, &mut passed_args, &mut program_args_started)?;
        result.process_color_args(color, passed_args, cargo_command, program_args_started);

        Ok(result)
    }

    fn process_main_args(
        &mut self,
        color: &mut String,
        passed_args: &mut impl Iterator<Item = String>,
        program_args_started: &mut bool,
    ) -> Result<()> {
        while let Some(arg) = passed_args.next() {
            if arg == "-h" || arg == "--help" {
                self.help = true;
                self.cargo_args.push(arg);
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
    ) {
        let terminal_supports_colors = atty::is(atty::Stream::Stderr);
        if self.short_message_format {
            self.cargo_args.push(MESSAGE_FORMAT_JSON_SHORT.to_owned());
        } else if !self.json_message_format {
            let message_format_arg = if color == COLOR_AUTO {
                if terminal_supports_colors {
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

        let command_supports_color_arg = cargo_command == "test";
        if command_supports_color_arg && !program_color_is_set && terminal_supports_colors {
            self.add_color_arg(COLOR_ALWAYS);
        }
    }

    fn parse_var<T: FromStr>(key: &str, default: &str) -> Result<T>
    where
        <T as FromStr>::Err: std::error::Error + Sync + Send + 'static,
    {
        Ok(env::var(key)
            .or_else(|_| Ok::<_, Error>(default.to_owned()))?
            .parse()
            .context(format!("invalid {} value", key))?)
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
