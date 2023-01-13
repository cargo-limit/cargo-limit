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
        const ESCAPE_CHAR: &str = "%";

        let mut raw_editor_data = String::new();
        input.read_to_string(&mut raw_editor_data)?;
        let command = format!(r#"call g:CargoLimitOpen({})"#, raw_editor_data);

        let editor_data: EditorData = serde_json::from_str(&raw_editor_data)?;
        let escaped_workspace_root = editor_data
            .workspace_root()
            .to_string_lossy()
            .replace(['/', '\\', ':'], ESCAPE_CHAR);

        Ok(Some(Self {
            escaped_workspace_root,
            command,
        }))
    }

    fn run(self) -> Result<Option<ExitStatus>> {
        let NeovimCommand {
            escaped_workspace_root,
            command,
        } = self;

        let server_name = nvim_listen_address(escaped_workspace_root)?;
        let nvim_send_args = vec!["--servername", &server_name, "--command", &command];

        match Command::new("nvim-send").args(nvim_send_args).output() {
            Ok(Output {
                status,
                stdout,
                stderr,
            }) => {
                let mut stdout_writer = io::stdout();
                stdout_writer.write_all(&stdout)?;
                stdout_writer.flush()?;

                let mut stderr_writer = io::stderr();
                stderr_writer.write_all(&stderr)?;
                stderr_writer.flush()?;

                Ok(Some(status))
            },
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(Error::from(err)),
        }
    }
}

fn nvim_listen_address(escaped_workspace_root: String) -> Result<String> {
    const PREFIX: &str = "nvim-cargo-limit-";

    let result = {
        #[cfg(windows)]
        {
            format!(
                r"\\.\pipe\{}{}-{}",
                PREFIX,
                env::var("USERNAME")?,
                escaped_workspace_root
            )
        }

        #[cfg(unix)]
        {
            format!(
                "/tmp/{}{}/{}",
                PREFIX,
                env::var("USER")?,
                escaped_workspace_root
            )
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
        if let Some(status) = neovim_command.run()? {
            status.code().unwrap_or(NO_EXIT_CODE)
        } else {
            NO_EXIT_CODE
        }
    } else {
        0
    };
    exit(code);
}
