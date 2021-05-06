# Neovim integration
`cargo-limit` can run an external application, providing it a list of files, lines and columns as arguments in the following format:

```
path/to/file1.rs:10:5 path/to/file2.rs:4:1
```

Theoretically this can be used for any text editor or IDE which supports client/server communication. In order to do that you need a wrapper script which parses the list and gives it to the text editor or IDE client.

## Installation
1. Run `pip3 install --user neovim-remote` and check that `nvr --version` runs without errors

2. Put a file called `open-in-nvim` somewhere in your `$PATH`:
```bash
#!/bin/sh

pwd_escaped=$(pwd | sed 's!/!%!g')
files=( "$@" )

# open files in reversed order
cmd=''
for ((i=${#files[@]}-1; i>=0; i--)); do
    item="${files[$i]}"
    filename=$(echo "${item}" | cut -d':' -f1)
    line=$(echo "${item}" | cut -d':' -f2)
    column=$(echo "${item}" | cut -d':' -f3)
    cmd+="<esc>:tab drop ${filename}<cr>${line}G${column}|"
done

NVIM_LISTEN_ADDRESS="/tmp/nvim-${pwd_escaped}" nvr -s --nostart --remote-send "${cmd}"
```

3. Add a file called `vi` to your `$PATH`:
```bash
#!/bin/sh

pwd_escaped=$(pwd | sed 's!/!%!g')
export NVIM_LISTEN_ADDRESS="/tmp/nvim-${pwd_escaped}"

# if can't connect then remove the socket
nvr -s --nostart || rm -f "${NVIM_LISTEN_ADDRESS}"

/usr/bin/nvim -p "$@"
```

4. `chmod +x open-in-nvim vi`

5. Set `CARGO_OPEN=open-in-nvim` environment variable

6. Open two terminals
- run `cd to/your/project ; vi` in one of them
- run `cd to/your/project ; cargo lcheck` in the other
    - optionally set `CARGO_MSG_LIMIT=1` if you want to open at most 1 file automatically
    - set `CARGO_OPEN_WARN=true` if you want to open files not just on errors but on warnings as well

For each file affected by error or warning Neovim will
- open it in new or existing tab
- jump to the corresponding line and column
