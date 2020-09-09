use anyhow::Result;
use cargo_fatal::{run_cargo, run_cargo_filtered, MESSAGE_FORMAT};
use std::{env, iter::once, process::exit};

fn main() -> Result<()> {
    let args = &["build", MESSAGE_FORMAT];
    let mut exit_code = run_cargo_filtered(args, 1, false)?;

    let success = exit_code == 0;
    if success {
        let args = once("run".to_owned()).chain(env::args().skip(1));
        exit_code = run_cargo(args)?;
    }

    exit(exit_code)
}
