use cargo_fatal::execute_cargo_twice_and_exit;
use std::env;
use std::iter;

const BUILD: &str = "build";
const ENABLE_COLORS: &str = "--color=always";

fn main() {
    let first_arguments = iter::once(BUILD.to_owned()).chain(iter::once(ENABLE_COLORS.to_owned()));
    let second_arguments = iter::once(BUILD.to_owned())
        .chain(iter::once(ENABLE_COLORS.to_owned()))
        .chain(env::args().skip(1));
    execute_cargo_twice_and_exit(first_arguments, second_arguments);
}
