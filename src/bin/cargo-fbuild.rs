use anyhow::Result;
use cargo_fatal::{run_cargo_filtered, MESSAGE_FORMAT};
use std::process::exit;

fn main() -> Result<()> {
    exit(run_cargo_filtered(&["build", MESSAGE_FORMAT], 1, true)?)
}
