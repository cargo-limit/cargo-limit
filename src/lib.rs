//! **Documentation is [here](https://github.com/alopatindev/cargo-limit#readme).**

#[doc(hidden)]
pub mod models;

mod cargo_toml;
mod io;
mod messages;
mod options;
mod process;

#[doc(hidden)]
pub use process::NO_EXIT_CODE;

use anyhow::Result;
use cargo_metadata::MetadataCommand;
use io::Buffers;
use messages::{MessageProcessor, ParsedMessages};

use options::Options;
use process::CargoProcess;
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

const ADDITIONAL_ENVIRONMENT_VARIABLES: &str =
    include_str!("../additional_environment_variables.txt");

#[doc(hidden)]
pub fn run_cargo_filtered(current_exe: String) -> Result<i32> {
    let workspace_root = MetadataCommand::new().no_deps().exec()?.workspace_root;
    let workspace_root = workspace_root.as_ref();
    let options = Options::from_os_env(current_exe, workspace_root)?;

    let mut cargo_process = CargoProcess::run(&options)?;

    let mut buffers = Buffers::new(cargo_process.child_mut())?;
    let mut parsed_messages =
        ParsedMessages::parse_with_timeout_on_error(&mut buffers, Some(&cargo_process), &options)?;

    let exit_code = if parsed_messages.child_killed() {
        buffers.writeln_to_stdout("")?;
        let exit_code = cargo_process.wait()?;
        parsed_messages.merge(ParsedMessages::parse_with_timeout_on_error(
            &mut buffers,
            None,
            &options,
        )?);
        process_messages(&mut buffers, parsed_messages, &options, workspace_root)?;
        buffers.copy_from_child_stdout_reader_to_stdout_writer()?;

        exit_code
    } else {
        process_messages(&mut buffers, parsed_messages, &options, workspace_root)?;
        buffers.copy_from_child_stdout_reader_to_stdout_writer()?;
        cargo_process.wait()?
    };

    if options.help() {
        buffers.write_to_stdout(ADDITIONAL_ENVIRONMENT_VARIABLES)?;
    }

    Ok(exit_code)
}

// TODO: move to MessageProcessor?
fn process_messages(
    buffers: &mut Buffers,
    parsed_messages: ParsedMessages,
    options: &Options,
    workspace_root: &Path,
) -> Result<()> {
    MessageProcessor::process(buffers, parsed_messages, &options, workspace_root)
}

#[doc(hidden)]
#[macro_export]
macro_rules! run_subcommand {
    () => {
        #[doc(hidden)]
        fn main() -> anyhow::Result<()> {
            use anyhow::Context;
            let current_exe = std::env::current_exe()?
                .file_stem()
                .context("invalid executable")?
                .to_string_lossy()
                .to_string();
            std::process::exit(cargo_limit::run_cargo_filtered(current_exe)?);
        }
    };
}
