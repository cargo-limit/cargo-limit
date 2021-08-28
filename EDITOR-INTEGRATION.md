# Neovim integration
Enable the plugin in your `~/.config/nvim/init.vim`. For instance for [vim-plug](https://github.com/junegunn/vim-plug#neovim):
```viml
Plug 'alopatindev/cargo-limit', { 'branch': 'nvim-plugin', 'do': 'cargo install --force --git https://github.com/alopatindev/nvim-send' }
```
and install it with `nvim +PlugInstall +UpdateRemotePlugins +qa`

## Testing
1. Open two terminals
2. `cd your/project/directory` in both of them
3. Run `nvim` in one of them
4. Run `cargo lrun` in the other
5. In case of compiling error `nvim` will open new or existing tabs with the files on affected lines and columns
6. `cargo llrun` (`cargo llcheck`, etc.) will make `nvim` open them in case of warnings as well.

# Other text editors/IDEs integration
cargo-limit can run an external app, providing it a project path and a list of affected files, lines and columns as arguments in the following format:

```
/full/path/to/project relative/path/to/file1.rs:10:5 relative/path/to/file2.rs:4:1
```

TODO: maybe send json instead? these args don't work with weird characters and/or spaces?
TODO: use stdin instead of args?

Theoretically this can be used for any text editor or IDE, especially if it supports client/server communication. In order to do that you need a wrapper script that parses the list and gives it to the text editor or IDE client.

## Example: gedit
1. Create `open-in-gedit.sh`:
```bash
#!/bin/bash

workspace_root="$1"
shift
files=( "$@" )
cmd=''
for ((i=${#files[@]}-1; i>=0; i--)); do
    item="${files[$i]}"
    filename=$(echo "${item}" | cut -d':' -f1)
    filename=$(printf "${workspace_root}/%q" "${filename}")
    line=$(echo "${item}" | cut -d':' -f2)
    column=$(echo "${item}" | cut -d':' -f3)
    gedit "${filename}" "+${line}:${column}" &
done
```
2. `chmod +x open-in-gedit.sh`
3. Set `CARGO_EDITOR=/path/to/open-in-gedit.sh` environment variable
4. Run `cargo lrun` in your project directory, in case of compiling error it will open files in `gedit` on affected lines and columns
5. `cargo llrun` (`cargo llcheck`, etc.) will open them in case of warnings as well.
