use anyhow::{Error, Result};
use cargo_limit::{
    models::{EditorData, SourceFile},
    NO_EXIT_CODE,
};
use std::{
    env, io,
    io::{Read, Write},
    process::{exit, Command, ExitStatus, Output},
};

// TODO: rename?
struct NeovimRemote {
    escaped_workspace_root: String,
    nvim_command: String,
}

fn escape_for_neovim_command(path: &str) -> String {
    path.replace(r"\", r"\\")
        .replace(r#"""#, r#"\""#)
        .replace("'", r"\'")
        .replace("[", r"\[")
        .replace("<", r"<LT>")
        .replace(" ", r"\ ")
}

impl NeovimRemote {
    fn from_editor_data<R: Read>(input: R) -> Result<Option<Self>> {
        const ESCAPE_CHAR: &str = "%";

        let editor_data: EditorData = serde_json::from_reader(input)?;
        let escaped_workspace_root = editor_data
            .workspace_root
            .to_string_lossy()
            .replace('/', ESCAPE_CHAR)
            .replace('\\', ESCAPE_CHAR)
            .replace(':', ESCAPE_CHAR);

        let mut command = Vec::new();
        for i in editor_data.files.into_iter() {
            let SourceFile {
                relative_path,
                line,
                column,
            } = i;
            let full_path = editor_data.workspace_root.join(relative_path);
            let escaped_full_path = escape_for_neovim_command(&full_path.to_string_lossy());
            command.push("<esc>:tab drop ".to_owned());
            command.push(escaped_full_path);
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

        // TODO: extract
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

        let nvim_send_args = vec![
            "--servername",
            &nvim_listen_address,
            "--remote-send",
            &nvim_command,
        ];

        match Command::new("nvim-send").args(nvim_send_args).output() {
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
    let code = if let Some(neovim_remote) = NeovimRemote::from_editor_data(&mut io::stdin())? {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let input = r###"/tmp/ ss z^_+<>,'=+@;]["11\z /asdf"###;
        let expected = r###"/tmp/\ ss\ z^_+<LT>>,\'=+@;]\[\"11\\z\ /asdf"###;
        assert_eq!(escape_for_neovim_command(input), expected);
    }
}

// TODO: write test?
