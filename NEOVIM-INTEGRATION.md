# Neovim integration
`cargo-limit` can run an external app, providing it a list of affected files, lines and columns as arguments in the following format:

```
/full/path/to/project relative/path/to/file1.rs:10:5 relative/path/to/file2.rs:4:1
```

Theoretically this can be used for any text editor or IDE which supports client/server communication. In order to do that you need a wrapper script that parses the list and gives it to the text editor or IDE client.

## Installation
1. Add to your `~/.config/nvim/init.vim`:
```viml
"TODO: use local vars
"TODO: detect whether cargo installed
let rust_project_dir = trim(system('cargo metadata --quiet --format-version=1 2>>/dev/null | jq --raw-output ".workspace_root"'))
if len(rust_project_dir) > 0
  "TODO: escape paths with spaces and weird characters
  let escaped_rust_project_dir = substitute(rust_project_dir, '/', '%', 'g')
  "TODO: detect OS, set named pipe on windows
  "TODO: escape windows username?
  "TODO: add cargo-limit text
  let socket_path = '/tmp/nvim-' . $USER . '-' . escaped_rust_project_dir
  if !filereadable(socket_path)
    call serverstart(socket_path)
  endif
endif
```

2. Run `pip3 install --user neovim-remote` and check that `nvr --version` runs without errors (`$HOME/.local/bin` should be listed in your `$PATH`)

3. Install [`jq`](https://stedolan.github.io/jq/download/)

4. Put a file called `open-in-nvim` somewhere in your `$PATH`:
```bash
#!/bin/bash

project_dir="$1"
project_dir_escaped=$(echo "$1" | sed 's!/!%!g')
nvim_listen_address="/tmp/nvim-${USER}-${project_dir_escaped}"

shift
files=( "$@" )
cmd=''
for ((i=${#files[@]}-1; i>=0; i--)); do
    item="${files[$i]}"
    filename=$(echo "${item}" | cut -d':' -f1)
    filename=$(printf "${project_dir}/%q" "${filename}")
    line=$(echo "${item}" | cut -d':' -f2)
    column=$(echo "${item}" | cut -d':' -f3)
    cmd+="<esc>:tab drop ${filename}<cr>${line}G${column}|"
done

nvr -s --nostart --servername "${nvim_listen_address}" --remote-send "${cmd}"
```

5. `chmod +x open-in-nvim`

6. Set `CARGO_OPEN=open-in-nvim` environment variable

7. Open two terminals
- run `cd to/your/project ; vi` in one of them
- run `cd to/your/project ; cargo lcheck` in the other

## Result
For each file affected by error (or warning as well, in case of running `cargo llcheck`) Neovim will:
- open it in new or existing tab
- jump to the corresponding line and column
