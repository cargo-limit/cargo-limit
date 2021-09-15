# Neovim integration
Enable the plugin in your `init.vim`. For instance for [vim-plug](https://github.com/junegunn/vim-plug#neovim):
```viml
Plug 'alopatindev/cargo-limit', { 'do': 'cargo install nvim-send' }
```
and install it with `nvim +PlugInstall +UpdateRemotePlugins +qa`

## Testing
1. Open two terminals
2. `cd your/project/directory` in both of them
3. Run `nvim` in one of them
4. Run `cargo lrun` in the other
5. In case of compiling errors `nvim` will open new or existing tabs with the files on affected lines and columns
6. `cargo llrun` (`cargo llcheck`, etc.) will open them in case of warnings as well.

# Other text editors/IDEs integration
cargo-limit can run external app/script and provide affected files in stdin in the following format:
```json
{
  "workspace_root": "/full/path/to/project",
  "files": [
    {
      "relative_path": "relative/path/to/file.rs",
      "line": 4,
      "column": 1
    }
  ]
}
```

Theoretically this can be used for any text editor or IDE, especially if it supports client/server communication. In order to do that you need a wrapper script that parses the files and gives it to the text editor or IDE client.

## Example: gedit
1. Install [`jq`](https://stedolan.github.io/jq/download/)
2. Create `open-in-gedit.sh`:
```bash
#!/bin/bash

jq --raw-output '. as $root | $root | .files[] | [
    "gedit",
    $root.workspace_root + "/" + .relative_path,
    "+" + (.line | tostring) + ":" + (.column | tostring),
    "&"
] | join(" ")' | bash
```
3. `chmod +x open-in-gedit.sh`
4. Set `CARGO_EDITOR=/path/to/open-in-gedit.sh` environment variable
5. Run `cargo lrun` in your project directory
6. In case of compiling errors `open-in-gedit.sh` will open files in `gedit` on affected lines and columns
7. `cargo llrun` (`cargo llcheck`, etc.) will open them in case of warnings as well.
