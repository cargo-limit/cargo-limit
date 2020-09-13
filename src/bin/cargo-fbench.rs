use anyhow::Result;
use cargo_fatal::run_cargo_filtered;
use std::process::exit;

fn main() -> Result<()> {
    exit(run_cargo_filtered("bench")?);
}
