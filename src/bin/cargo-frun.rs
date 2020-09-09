use anyhow::Result;
use cargo_fatal::{run_cargo, run_cargo_filtered, MESSAGE_FORMAT};
use std::process::exit;

fn main() -> Result<()> {
    // TODO: additioanl build args
    // TODO: additional app args
    let mut exit_code = run_cargo_filtered(&["build", MESSAGE_FORMAT], 1, false)?;
    let success = exit_code == 0;
    if success {
        exit_code = run_cargo(&["run"])?;
    }
    exit(exit_code)
}
