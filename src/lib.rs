use std::env;
use std::ffi::OsString;
use std::io::{self, Write};
use std::iter;
use std::process::{Command, ExitStatus};

const CARGO: &str = "cargo";
const RUSTFLAGS: &str = "RUSTFLAGS";
const IGNORE_WARNINGS: &str = "-A warnings";

pub fn execute_cargo(
    arguments: impl Iterator<Item = String>,
    environment_variables: impl Iterator<Item = (String, String)>,
) -> ExitStatus {
    let output = Command::new(CARGO)
        .args(arguments)
        .envs(environment_variables)
        .output()
        .unwrap();
    io::stderr().write_all(&output.stderr).unwrap();
    io::stdout().write_all(&output.stdout).unwrap();

    output.status
}

pub fn execute_cargo_without_warnings(arguments: impl Iterator<Item = String>) -> ExitStatus {
    let flags = env::var_os(RUSTFLAGS)
        .unwrap_or_else(OsString::new)
        .into_string()
        .unwrap();
    let flags = if flags.is_empty() {
        IGNORE_WARNINGS.to_owned()
    } else {
        format!("{} {}", flags, IGNORE_WARNINGS)
    };

    let environment_variables = iter::once((RUSTFLAGS.to_owned(), flags));
    execute_cargo(arguments, environment_variables)
}
