use anyhow::Result;
use cargo_fatal::{run_cargo, MESSAGE_FORMAT, NO_RUN};

fn main() -> Result<()> {
    run_cargo(&["test", NO_RUN, MESSAGE_FORMAT], 1)
}
