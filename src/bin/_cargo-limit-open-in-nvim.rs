use anyhow::{Context, Error, Result};
use cargo_limit::NO_EXIT_CODE;
use std::{
    env, io,
    io::Write,
    path::PathBuf,
    process::{exit, Command, ExitStatus, Output},
    str::FromStr,
};

struct SourceFile {
    relative_path: PathBuf,
    line: usize,
    column: usize,
}

struct NeovimRemote {
    escaped_workspace_root: String,
    nvim_command: String,
}

impl FromStr for SourceFile {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut iter = text.rsplitn(3, ':').collect::<Vec<_>>().into_iter().rev();
        let relative_path = next(&mut iter)?.parse()?;
        let line = next(&mut iter)?.parse()?;
        let column = next(&mut iter)?.parse()?;
        Ok(Self {
            relative_path,
            line,
            column,
        })
    }
}

fn next<T>(iter: &mut impl Iterator<Item = T>) -> Result<T> {
    iter.next().context("invalid arguments")
}

impl NeovimRemote {
    fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Option<Self>> {
        const ESCAPE_CHAR: &str = "%";

        let _ = args.next();
        let mut args = args.peekable();
        if args.peek().is_none() {
            return Ok(None);
        }

        let workspace_root = next(&mut args)?.parse::<PathBuf>()?;

        let escaped_workspace_root = workspace_root
            .to_string_lossy()
            .replace('/', ESCAPE_CHAR)
            .replace('\\', ESCAPE_CHAR)
            .replace(':', ESCAPE_CHAR);

        let mut command = Vec::new();
        for i in args.collect::<Vec<_>>().into_iter().rev() {
            let SourceFile {
                relative_path,
                line,
                column,
            } = i.parse()?;
            let full_path = workspace_root.join(relative_path);

            command.push("<esc>:tab drop ".to_owned());
            command.push(full_path.to_string_lossy().to_string());
            command.push("<cr>".to_owned());
            command.push(line.to_string());
            command.push("G".to_owned());
            command.push(column.to_string());
            command.push("|".to_owned());
        }

        let nvim_command = command.join("");
        Ok(Some(Self {
            escaped_workspace_root,
            nvim_command,
        }))
    }

    fn run(self) -> Result<Option<ExitStatus>> {
        const PREFIX: &str = "nvim-cargo-limit-";

        let NeovimRemote {
            escaped_workspace_root,
            nvim_command,
        } = self;

        let nvim_listen_address = {
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

        let nvr_args = vec![
            "--servername",
            &nvim_listen_address,
            "--remote-send",
            &nvim_command,
        ];

        match Command::new("nvim-send").args(nvr_args).output() {
            Ok(Output {
                status,
                stdout,
                stderr,
            }) => {
                let mut stdout_writer = io::stdout();
                stdout_writer.write(&stdout)?;
                stdout_writer.flush()?;

                let mut stderr_writer = io::stderr();
                stderr_writer.write(&stderr)?;
                stderr_writer.flush()?;

                Ok(Some(status))
            },
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(Error::from(err)),
        }
    }
}

fn main() -> Result<()> {
    let code = if let Some(neovim_remote) = NeovimRemote::parse_args(env::args())? {
        if let Some(status) = neovim_remote.run()? {
            status.code().unwrap_or(NO_EXIT_CODE)
        } else {
            NO_EXIT_CODE // TODO: or 0? or something else?
        }
    } else {
        0
    };
    exit(code);
}

// TODO: write test?
