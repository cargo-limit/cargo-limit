# Neovim integration
`cargo-limit` can run an external application, providing it a list of affected files, lines and columns as arguments in the following format:

```
/full/path/to/project relative/path/to/file1.rs:10:5 relative/path/to/file2.rs:4:1
```

Theoretically this can be used for any text editor or IDE which supports client/server communication. In order to do that you need a wrapper script that parses the list and gives it to the text editor or IDE client.

## Installation
1. Run `pip3 install --user neovim-remote` and check that `nvr --version` runs without errors (`$HOME/.local/bin` should be listed in your `$PATH`)

2. Install [`jq`](https://stedolan.github.io/jq/download/)

3. Put a file called `open-in-nvim` somewhere in your `$PATH`:
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

4. Add a file called `vi` to your `$PATH`:
```bash
#!/bin/bash

project_dir=$(cargo metadata --quiet --format-version=1 2>>/dev/null | jq --raw-output '.workspace_root')
project_dir_escaped=$(echo "${project_dir}" | sed 's!/!%!g')
nvim_listen_address="/tmp/nvim-${USER}-${project_dir_escaped}"

/usr/bin/nvim --listen "${nvim_listen_address}" -p "$@"
```

5. `chmod +x open-in-nvim vi`

6. Set `CARGO_OPEN=open-in-nvim` environment variable

7. Open two terminals
- run `cd to/your/project ; vi` in one of them
- run `cd to/your/project ; cargo lcheck` in the other
    - optionally set `CARGO_MSG_LIMIT=1` if you want to open at most 1 file automatically

## Result
For each file affected by error (or warning as well, in case of running `cargo llcheck`) Neovim will:
- open it in new or existing tab
- jump to the corresponding line and column
