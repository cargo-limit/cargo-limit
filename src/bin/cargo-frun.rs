use anyhow::Result;
use cargo_fatal::{run_cargo_filtered, MESSAGE_FORMAT, RUN};
use std::process::exit;

fn main() -> Result<()> {
    exit(run_cargo_filtered(&[RUN, MESSAGE_FORMAT])?);
}
