use anyhow::Result;
use cargo_fatal::{prepare_args, run_cargo, run_cargo_filtered, MESSAGE_FORMAT};
use std::process::exit;

fn main() -> Result<()> {
    let args = prepare_args(&["build", MESSAGE_FORMAT]);
    let mut exit_code = run_cargo_filtered(args, 1, false)?;

    let success = exit_code == 0;
    if success {
        let args = prepare_args(&["run"]);
        exit_code = run_cargo(args)?;
    }

    exit(exit_code)
}
