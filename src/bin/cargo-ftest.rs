use anyhow::Result;
use cargo_fatal::{run_cargo, run_cargo_filtered, MESSAGE_FORMAT, NO_RUN};
use std::{env, iter::once, process::exit};

fn main() -> Result<()> {
    let args = &["test", MESSAGE_FORMAT, NO_RUN];
    let mut exit_code = run_cargo_filtered(args, 1, false)?;

    let success = exit_code == 0;
    if success {
        let args = once("test".to_owned()).chain(env::args().skip(1));
        exit_code = run_cargo(args)?;
    }

    exit(exit_code)
}
