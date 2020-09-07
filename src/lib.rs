use std::env;
use std::ffi::OsString;
use std::io::{self, Write};
use std::iter;
use std::process::{exit, Command, ExitStatus};

const CARGO: &str = "cargo";
const RUSTFLAGS: &str = "RUSTFLAGS";
const IGNORE_WARNINGS: &str = "-A warnings";
const NO_STATUS_CODE: i32 = 127;

pub fn execute_cargo_twice_and_exit(
    first_arguments: impl Iterator<Item = String>,
    second_arguments: impl Iterator<Item = String>,
) {
    let status = execute_cargo_without_warnings(first_arguments);
    let mut status_code = status.code().unwrap_or(NO_STATUS_CODE);

    if status.success() {
        let status = execute_cargo(second_arguments, iter::empty());
        status_code = status.code().unwrap_or(NO_STATUS_CODE);
    }

    exit(status_code);
}

fn execute_cargo(
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

fn execute_cargo_without_warnings(arguments: impl Iterator<Item = String>) -> ExitStatus {
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
