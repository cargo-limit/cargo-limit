# Neovim
See [here](../#neovim-integration).

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
4. Run `cargo lrun` in your project, it will open `gedit` on the affected line and column in case of compiling error

`cargo llrun` will open it in case of warnings as well.
