use anyhow::Result;
use cargo_fatal::{prepare_args, run_cargo_filtered, MESSAGE_FORMAT, TEST};
use std::process::exit;

fn main() -> Result<()> {
    let args = prepare_args(&[TEST, MESSAGE_FORMAT]);
    exit(run_cargo_filtered(args, 1)?);
}
