use anyhow::Result;
use cargo_fatal::{run_cargo_filtered, MESSAGE_FORMAT, TEST};
use std::process::exit;

fn main() -> Result<()> {
    exit(run_cargo_filtered(&[TEST, MESSAGE_FORMAT], 1)?);
}
