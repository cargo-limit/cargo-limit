use anyhow::{Error, Result};
use cargo_limit::{models::EditorData, NO_EXIT_CODE};
use std::{
    env, io,
    io::{Read, Write},
    process::{exit, Command, ExitStatus, Output},
};

struct NeovimCommand {
    escaped_workspace_root: String,
    command: String,
}

impl NeovimCommand {
    fn from_editor_data<R: Read>(mut input: R) -> Result<Option<Self>> {
        let mut raw_editor_data = String::new();
        input.read_to_string(&mut raw_editor_data)?;
        let command = format!(r#"<ESC>:call g:CargoLimitOpen({raw_editor_data})<Enter>"#);

        let editor_data: EditorData = serde_json::from_str(&raw_editor_data)?;
        let escaped_workspace_root = editor_data.escaped_workspace_root();

        Ok(Some(Self {
            escaped_workspace_root,
            command,
        }))
    }

    fn run(self) -> Result<ExitStatus> {
        let NeovimCommand {
            escaped_workspace_root,
            command,
        } = self;

        let server_name = nvim_listen_address(escaped_workspace_root)?;
        let remote_send_args = vec!["--server", &server_name, "--remote-send", &command];

        match Command::new("nvim").args(remote_send_args).output() {
            Ok(Output {
                status,
                stdout,
                stderr,
            }) => {
                let mut stdout_writer = io::stdout();
                stdout_writer.write_all(&stdout)?;
                stdout_writer.flush()?;

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

fn nvim_listen_address(escaped_workspace_root: String) -> Result<String> {
    const PREFIX: &str = "nvim-cargo-limit-";

    let result = {
        #[cfg(windows)]
        {
            let user = env::var("USERNAME")?;
            format!(r"\\.\pipe\{PREFIX}{user}-{escaped_workspace_root}")
        }

        #[cfg(unix)]
        {
            let user = env::var("USER")?;
            format!("/tmp/{PREFIX}{user}/{escaped_workspace_root}")
        }

        #[cfg(not(any(unix, windows)))]
        {
            compile_error!("this platform is unsupported")
        }
    };

    Ok(result)
}

fn main() -> Result<()> {
    let code = if let Some(neovim_command) = NeovimCommand::from_editor_data(&mut io::stdin())? {
        neovim_command.run()?.code().unwrap_or(NO_EXIT_CODE)
    } else {
        0
    };
    exit(code);
}
