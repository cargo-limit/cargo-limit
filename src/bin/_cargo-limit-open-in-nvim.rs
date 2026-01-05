use anyhow::{Error, Result};
use cargo_limit::{NO_EXIT_CODE, env_vars, models::EditorData};
use std::{
    env, io,
    io::{Read, Write},
    process::{Command, ExitStatus, Output, exit},
};

#[doc(hidden)]
struct NeovimCommand {
    escaped_workspace_root: String,
    command: String,
}

impl NeovimCommand {
    fn new(command: &str, raw_editor_data: &str) -> Result<Self> {
        let command = format!(r#"{command}({raw_editor_data})"#);

        let editor_data: EditorData = serde_json::from_str(raw_editor_data)?;
        let escaped_workspace_root = editor_data.escaped_workspace_root();

        Ok(Self {
            escaped_workspace_root,
            command,
        })
    }

    fn run(self) -> Result<ExitStatus> {
        let server_name = nvim_listen_address(self.escaped_workspace_root)?;
        let remote_send_args = vec![
            "--headless",
            "--clean",
            "--server",
            &server_name,
            "--remote-expr",
            &self.command,
        ];

        match Command::new("nvim").args(remote_send_args).output() {
            Ok(Output {
                status,
                stdout,
                stderr,
            }) => {
                const EXPECTED_EXPR_RESULT: [u8; 1] = [b'0'];
                if stdout != EXPECTED_EXPR_RESULT {
                    let mut stdout_writer = io::stdout();
                    stdout_writer.write_all(&stdout)?;
                    stdout_writer.flush()?;
                }

                let failed_to_connect_is_the_only_error = stderr.starts_with(b"E247:")
                    && stderr.iter().filter(|i| **i == b'\n').count() == 1;
                if !failed_to_connect_is_the_only_error {
                    let mut stderr_writer = io::stderr();
                    stderr_writer.write_all(&stderr)?;
                    stderr_writer.flush()?;
                }

                Ok(status)
            },
            Err(err) => Err(Error::from(err)),
        }
    }
}

#[doc(hidden)]
fn nvim_listen_address(escaped_workspace_root: String) -> Result<String> {
    const PREFIX: &str = "nvim-cargo-limit-";

    let result = {
        let user = env::var(env_vars::USER)?;

        #[cfg(unix)]
        {
            format!("/tmp/{PREFIX}{user}/{escaped_workspace_root}")
        }

        #[cfg(windows)]
        {
            format!(r"\\.\pipe\{PREFIX}{user}-{escaped_workspace_root}")
        }

        #[cfg(not(any(unix, windows)))]
        {
            compile_error!("this platform is unsupported")
        }
    };

    Ok(result)
}

#[doc(hidden)]
fn main() -> Result<()> {
    let mut raw_editor_data = String::new();
    io::stdin().read_to_string(&mut raw_editor_data)?;

    let command = NeovimCommand::new("g:CargoLimitOpen", &raw_editor_data)?;
    exit(command.run()?.code().unwrap_or(NO_EXIT_CODE));
}
