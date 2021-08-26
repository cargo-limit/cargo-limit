use anyhow::{format_err, Error, Result};
use std::{env, path::PathBuf, str::FromStr};

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

// TODO: is it used somewhere else?
fn next<T>(iter: &mut impl Iterator<Item = T>) -> Result<T> {
    iter.next().ok_or_else(|| format_err!("invalid arguments"))
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
        for i in args {
            let SourceFile {
                relative_path,
                line,
                column,
            } = i.parse()?;
            let full_path = workspace_root.join(relative_path);
            command.push("<esc>:tab drop ");
            command.push(&full_path.to_string_lossy());
            command.push("<cr>");
            command.push(line.to_string().as_str());
            command.push("G");
            command.push(column.to_string().as_str());
            command.push("|");
        }

        let nvim_command = command.join("");
        Ok(Some(Self {
            escaped_workspace_root,
            nvim_command,
        }))
    }

    fn run(self) -> Result<()> {
        const PREFIX: &str = "nvim-cargo-limit-";

        let NeovimRemote {
            escaped_workspace_root,
            nvim_command,
        } = self;

        let nvim_listen_address = {
            #[cfg(windows)]
            {
                format!(
                    "\\\\.\\pipe\\{}{}-{}",
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

        let args = vec![
            "nvr",
            "-s",
            "--nostart",
            "--servername",
            &nvim_listen_address,
            "--remote-send",
            &nvim_command,
        ];
        dbg!(args);
        // TODO: run it here

        Ok(())
    }
}

fn main() -> Result<()> {
    if let Some(neovim_remote) = NeovimRemote::parse_args(env::args())? {
        neovim_remote.run()?;
    }
    Ok(())
}

// TODO: write test?
