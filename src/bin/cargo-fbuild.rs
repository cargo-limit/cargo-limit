use cargo_fatal::*;
use std::env;
use std::iter;
use std::process::exit;

const BUILD: &str = "build";
const ENABLE_COLORS: &str = "--color=always";

const NO_STATUS_CODE: i32 = 127;

fn main() {
    let arguments = iter::once(BUILD.to_owned()).chain(iter::once(ENABLE_COLORS.to_owned()));
    let status = execute_cargo_without_warnings(arguments);
    let mut status_code = status.code().unwrap_or(NO_STATUS_CODE);

    if status.success() {
        let arguments = iter::once(BUILD.to_owned())
            .chain(iter::once(ENABLE_COLORS.to_owned()))
            .chain(env::args().skip(1));
        let status = execute_cargo(arguments, iter::empty());
        status_code = status.code().unwrap_or(NO_STATUS_CODE);
    }

    exit(status_code);
}
