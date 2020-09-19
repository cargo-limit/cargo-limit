use anyhow::{Context, Error, Result};
use std::{env, str::FromStr};

const COLOR: &str = "--color=";
const PROGRAM_ARGS_DELIMITER: &str = "--";
const JSON_MESSAGE_FORMAT: &str = "--message-format=json";
const JSON_MESSAGE_FORMAT_WITH_COLORS: &str = "--message-format=json-diagnostic-rendered-ansi";

pub struct Options {
    pub cargo_args: Vec<String>,
    pub limit_messages: usize,
    pub ascending_messages_order: bool,
    pub show_warnings_if_errors_exist: bool,
    pub help: bool,
}

impl Options {
    pub fn from_args_and_vars(cargo_command: &str) -> Result<Self> {
        let mut passed_args = env::args().skip(2);
        let mut result = Self {
            cargo_args: Vec::new(),
            limit_messages: Self::parse_var("CARGO_LIMIT", "0")?,
            ascending_messages_order: Self::parse_var("CARGO_ASC", "false")?,
            show_warnings_if_errors_exist: Self::parse_var("CARGO_ALWAYS_SHOW_WARNINGS", "false")?,
            help: false,
        };
        let mut program_args_started = false;
        let mut color = "auto".to_owned();

        result.cargo_args.push(cargo_command.to_owned());

        while let Some(arg) = passed_args.next() {
            if arg == "-h" || arg == "--help" {
                result.help = true;
                result.cargo_args.push(arg);
            } else if arg == COLOR[0..COLOR.len() - 1] {
                color = passed_args.next().context(
                    "the argument '--color <WHEN>' requires a value but none was supplied",
                )?;
                Self::validate_color(&color)?;
            } else if arg.starts_with(COLOR) {
                color = arg[COLOR.len()..].to_owned();
                Self::validate_color(&color)?;
            } else if arg == PROGRAM_ARGS_DELIMITER {
                program_args_started = true;
                break;
            } else {
                result.cargo_args.push(arg);
            }
        }

        let terminal_supports_colors = atty::is(atty::Stream::Stdout);
        result.add_color_arg(&color);
        let message_format_arg = match color.as_str() {
            "auto" if terminal_supports_colors => JSON_MESSAGE_FORMAT_WITH_COLORS,
            "auto" if !terminal_supports_colors => JSON_MESSAGE_FORMAT,
            "always" => JSON_MESSAGE_FORMAT_WITH_COLORS,
            "never" => JSON_MESSAGE_FORMAT,
            _ => unreachable!(),
        };
        result.cargo_args.push(message_format_arg.to_owned());

        let mut program_color_is_set = false;
        if program_args_started {
            result.cargo_args.push(PROGRAM_ARGS_DELIMITER.to_owned());
            while let Some(arg) = passed_args.next() {
                if arg == COLOR[0..COLOR.len() - 1] || arg.starts_with(COLOR) {
                    program_color_is_set = true;
                }
                result.cargo_args.push(arg);
            }
        }

        if !program_args_started {
            result.cargo_args.push(PROGRAM_ARGS_DELIMITER.to_owned());
        }

        let command_supports_color_arg = cargo_command == "test";
        if command_supports_color_arg && !program_color_is_set {
            if terminal_supports_colors {
                result.add_color_arg("always");
            }
        }

        Ok(result)
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
        if !["auto", "always", "never"].contains(&color) {
            return Err(Error::msg(
                "argument for --color must be auto, always, or never",
            ));
        }
        Ok(())
    }

    fn add_color_arg(&mut self, value: &str) {
        self.cargo_args.push(format!("{}{}", COLOR, value));
    }
}
