use cargo_fatal::execute_cargo_twice_and_exit;
use std::env;
use std::iter;

const TEST: &str = "test";
const NO_RUN: &str = "--no-run";
const ENABLE_COLORS: &str = "--color=always";

fn main() {
    let first_arguments = iter::once(TEST.to_owned())
        .chain(iter::once(NO_RUN.to_owned()))
        .chain(iter::once(ENABLE_COLORS.to_owned()));
    let second_arguments = iter::once(TEST.to_owned())
        .chain(iter::once(ENABLE_COLORS.to_owned()))
        .chain(env::args().skip(1));
    execute_cargo_twice_and_exit(first_arguments, second_arguments);
}
