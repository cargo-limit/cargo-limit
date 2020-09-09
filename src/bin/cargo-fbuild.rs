use anyhow::Result;
use cargo_fatal::{run_cargo, MESSAGE_FORMAT};

fn main() -> Result<()> {
    run_cargo(&["build", MESSAGE_FORMAT], 1)
}
