use anyhow::{Context, Result};

const LIMIT_MESSAGES: &str = "--limit=";

pub struct ParsedArgs {
    pub cargo_args: Vec<String>,
    pub limit_messages: usize,
    pub ascending_messages_order: bool,
    pub help: bool,
}

impl ParsedArgs {
    pub fn parse(mut passed_args: impl Iterator<Item = String>) -> Result<Self> {
        let mut result = Self {
            cargo_args: Vec::new(),
            limit_messages: 0,
            ascending_messages_order: false,
            help: false,
        };
        let mut program_args_started = false;

        while let Some(arg) = passed_args.next() {
            if program_args_started {
                result.cargo_args.push(arg);
            } else if arg == "-h" || arg == "--help" {
                result.help = true;
                result.cargo_args.push(arg);
            } else if arg == LIMIT_MESSAGES[0..LIMIT_MESSAGES.len() - 1] {
                result.limit_messages = passed_args
                    .next()
                    .context("expected number of messages")?
                    .parse()?;
            } else if arg.starts_with(LIMIT_MESSAGES) {
                result.limit_messages = arg[LIMIT_MESSAGES.len()..].parse()?;
            } else if arg == "--asc" {
                result.ascending_messages_order = true;
            } else if arg == "--" {
                program_args_started = true;
                result.cargo_args.push(arg);
            } else {
                result.cargo_args.push(arg);
            }
        }

        Ok(result)
    }
}
