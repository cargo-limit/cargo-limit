use anyhow::Result;
use cargo_fatal::{prepare_args, run_cargo, run_cargo_filtered, BENCH, MESSAGE_FORMAT, NO_RUN};
use std::process::exit;

fn main() -> Result<()> {
    let args = prepare_args(&[BENCH, MESSAGE_FORMAT, NO_RUN]);
    let mut exit_code = run_cargo_filtered(args, 1, false)?;

    let success = exit_code == 0;
    if success {
        let args = prepare_args(&[BENCH]);
        exit_code = run_cargo(args)?;
    }

    exit(exit_code)
}
