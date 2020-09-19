use anyhow::{Context, Error, Result};

const COLOR: &str = "--color=";
const LIMIT_MESSAGES: &str = "--limit=";
const PROGRAM_ARGS_DELIMITER: &str = "--";

pub struct ParsedArgs {
    pub cargo_args: Vec<String>,
    pub limit_messages: usize,
    pub ascending_messages_order: bool,
    pub show_warnings_if_errors_exist: bool,
    pub help: bool,
}

impl ParsedArgs {
    pub fn parse(
        mut passed_args: impl Iterator<Item = String>,
        cargo_command: &str,
    ) -> Result<Self> {
        let mut result = Self {
            cargo_args: Vec::new(),
            limit_messages: 0,
            ascending_messages_order: false,
            show_warnings_if_errors_exist: false,
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
            } else if arg == LIMIT_MESSAGES[0..LIMIT_MESSAGES.len() - 1] {
                result.limit_messages = passed_args
                    .next()
                    .context("expected number of messages")?
                    .parse()?;
            } else if arg.starts_with(LIMIT_MESSAGES) {
                result.limit_messages = arg[LIMIT_MESSAGES.len()..].parse()?;
            } else if arg == "--asc" {
                result.ascending_messages_order = true;
            } else if arg == "--always-show-warnings" {
                result.show_warnings_if_errors_exist = true;
            } else if arg == PROGRAM_ARGS_DELIMITER {
                program_args_started = true;
                break;
            }
        }

        result.add_color_arg(&color);
        if color == "never" {
            result.cargo_args.push("--message-format=json".to_owned());
        } else {
            result
                .cargo_args
                .push("--message-format=json-diagnostic-rendered-ansi".to_owned());
        }

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
            if atty::is(atty::Stream::Stdout) {
                result.add_color_arg("always");
            }
        }

        Ok(result)
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
