use anyhow::Result;
use cargo_fatal::{prepare_args, run_cargo_filtered, BUILD, MESSAGE_FORMAT};
use std::process::exit;

fn main() -> Result<()> {
    let args = prepare_args(&[BUILD, MESSAGE_FORMAT]);
    exit(run_cargo_filtered(args, 1, true)?)
}
