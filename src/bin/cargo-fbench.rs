use anyhow::Result;
use cargo_fatal::{run_cargo_filtered, MESSAGE_FORMAT, NO_RUN};
use std::process::exit;

fn main() -> Result<()> {
    exit(run_cargo_filtered(
        &["bench", NO_RUN, MESSAGE_FORMAT],
        1,
        true,
    )?)
}
