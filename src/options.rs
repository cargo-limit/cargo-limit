use crate::cargo_toml::CargoToml;
use anyhow::{format_err, Context, Error, Result};
use clap::{App, AppSettings, Arg};
use const_format::concatcp;
use itertools::{repeat_n, Either, Itertools};
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
    pub open_in_external_application: String,
    pub open_in_external_application_on_warnings: bool,
    pub help: bool,
    pub version: bool,
    pub json_message_format: bool,
    pub short_message_format: bool,
}

impl Options {
    pub fn from_args_and_vars(workspace_root: &Path) -> Result<Self> {
        let mut passed_args = parse_and_reorder_args_with_clap(env::args().skip(1))?.into_iter();

        let mut result = Self {
            cargo_args: Vec::new(),
            limit_messages: Self::parse_var("CARGO_MSG_LIMIT", "0")?,
            time_limit_after_error: Duration::from_secs(Self::parse_var("CARGO_TIME_LIMIT", "1")?),
            ascending_messages_order: Self::parse_var("CARGO_ASC", "false")?,
            show_warnings_if_errors_exist: Self::parse_var("CARGO_FORCE_WARN", "false")?,
            show_dependencies_warnings: Self::parse_var("CARGO_DEPS_WARN", "false")?,
            open_in_external_application: Self::parse_var("CARGO_OPEN", "")?,
            open_in_external_application_on_warnings: Self::parse_var("CARGO_OPEN_WARN", "false")?,
            help: false,
            version: false,
            json_message_format: false,
            short_message_format: false,
        };
        let mut program_args_started = false;
        let mut color = COLOR_AUTO.to_owned();

        let first_arg = passed_args
            .next()
            .ok_or_else(|| format_err!("command not found"))?;
        let (first_letter, cargo_command) = first_arg.split_at(1);
        assert_eq!(first_letter, "l");
        result.cargo_args.push(cargo_command.to_owned()); // TODO: it's not really cargo_args anymore

        result.process_main_args(&mut color, &mut passed_args, &mut program_args_started)?;
        result.process_color_and_program_args(
            color,
            passed_args,
            &cargo_command,
            program_args_started,
            workspace_root,
        )?;

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
            } else if arg == "-v" || arg == "--version" {
                self.version = true;
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

    fn process_color_and_program_args(
        &mut self,
        color: String,
        passed_args: impl Iterator<Item = String>,
        cargo_command: &str,
        program_args_started: bool,
        workspace_root: &Path,
    ) -> Result<()> {
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

        let is_test = cargo_command == "test";
        let is_bench = cargo_command == "bench";
        let command_supports_color_arg = is_test || is_bench;
        if command_supports_color_arg && !program_color_is_set && terminal_supports_colors {
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

    fn parse_var<T: FromStr>(key: &str, default: &str) -> Result<T>
    where
        <T as FromStr>::Err: std::error::Error + Sync + Send + 'static,
    {
        env::var(key)
            .or_else(|_| Ok::<_, Error>(default.to_owned()))?
            .parse()
            .context(format!("invalid {} value", key))
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

fn parse_and_reorder_args_with_clap(args: impl Iterator<Item = String>) -> Result<Vec<String>> {
    let mut args = args.peekable();
    let app_name = args
        .peek()
        .ok_or_else(|| format_err!("command not found"))?;
    let app = App::new(app_name)
        .settings(&[
            AppSettings::UnifiedHelpMessage,
            AppSettings::DeriveDisplayOrder,
            AppSettings::VersionlessSubcommands,
            AppSettings::AllowExternalSubcommands,
        ])
        .arg(opt("version", "Print version info and exit").short("V"))
        .arg(opt("list", "List installed commands"))
        .arg(opt("explain", "Run `rustc --explain CODE`").value_name("CODE"))
        .arg(
            opt(
                "verbose",
                "Use verbose output (-vv very verbose/build.rs output)",
            )
            .short("v")
            .multiple(true)
            .global(true),
        )
        .arg(opt("quiet", "No output printed to stdout").short("q"))
        .arg(
            opt("color", "Coloring: auto, always, never")
                .value_name("WHEN")
                .global(true),
        )
        .arg(opt("frozen", "Require Cargo.lock and cache are up to date").global(true))
        .arg(opt("locked", "Require Cargo.lock is up to date").global(true))
        .arg(opt("offline", "Run without accessing the network").global(true))
        .arg(
            multi_opt(
                "config",
                "KEY=VALUE",
                "Override a configuration value (unstable)",
            )
            .global(true),
        )
        .arg(
            Arg::with_name("unstable-features")
                .help("Unstable (nightly-only) flags to Cargo, see 'cargo -Z help' for details")
                .short("Z")
                .value_name("FLAG")
                .multiple(true)
                .number_of_values(1)
                .global(true),
        );

    let cargo_command = app.get_name().to_owned();

    let app_matches = app.get_matches_from(args);

    let cargo_args = app_matches
        .args
        .into_iter()
        .flat_map(|(key, matched_arg)| {
            let missing_values = repeat_n(None, matched_arg.indices.len() - matched_arg.vals.len());
            matched_arg
                .indices
                .into_iter()
                .zip(matched_arg.vals.into_iter().map(Some).chain(missing_values))
                .map(move |(i, value)| (i, key, value))
        })
        .sorted_by_key(|(i, _, _)| *i)
        .map(|(_, key, value)| {
            let result = if let Some(value) = value {
                format!(
                    "--{}={}",
                    key,
                    value
                        .into_string()
                        .map_err(|_| format_err!("cannot convert argument value"))?
                )
            } else {
                format!("--{}", key) // TODO: extract to contant? or somehow convert using clap?
            };
            Ok(result)
        })
        .collect::<Result<Vec<_>>>()?;

    let program_args = app_matches
        .subcommand
        .map(|subcommand| {
            Either::Left(
                iter::once(Ok(subcommand.name)).chain(
                    subcommand.matches.clone().args[""]
                        .vals
                        .clone()
                        .into_iter()
                        .map(|i| {
                            i.into_string()
                                .map_err(|_| format_err!("cannot convert argument value"))
                        }),
                ),
            )
        })
        .unwrap_or(Either::Right(iter::empty()))
        .collect::<Result<Vec<_>>>()?;

    let all_args = iter::once(cargo_command)
        .chain(
            cargo_args
                .into_iter()
                .chain(iter::once("--".to_owned()))
                .chain(program_args),
        )
        .collect::<Vec<_>>();
    dbg!(&all_args);

    Ok(all_args)
}

fn opt(name: &'static str, help: &'static str) -> Arg<'static, 'static> {
    Arg::with_name(name).long(name).help(help)
}

fn multi_opt(
    name: &'static str,
    value_name: &'static str,
    help: &'static str,
) -> Arg<'static, 'static> {
    opt(name, help)
        .value_name(value_name)
        .multiple(true)
        .number_of_values(1)
}
