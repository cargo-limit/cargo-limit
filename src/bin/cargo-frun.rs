use anyhow::Result;
use cargo_fatal::{run_cargo, MESSAGE_FORMAT, NO_RUN};

fn main() -> Result<()> {
    run_cargo(&["build", MESSAGE_FORMAT], 1)

    // TODO: run the build
    // TODO: ensure there are no double warnings
}
