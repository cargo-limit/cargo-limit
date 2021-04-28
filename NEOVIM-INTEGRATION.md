# Neovim integration
`cargo-limit` can run an external application, providing it a list of files, lines and columns as arguments in the following format:

```
path/to/file1.rs:10:5 path/to/file2.rs:4:1
```

Theoretically this can be used for any text editor or IDE which supports remote control. In order to do that you need a wrapper script which parses the list and gives it to the text editor or IDE.

## Installation
1. Run `pip3 install --user neovim-remote` and check that `nvr --version` runs without errors

2. Put a file called `open-in-nvim-all` somewhere in your `$PATH`:
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

If you prefer to open the first file only then add a file called `open-in-nvim-first`:
```bash
#!/bin/sh

pwd_escaped=$(pwd | sed 's!/!%!g')
item="${1}"
filename=$(echo "${item}" | cut -d':' -f1)
line=$(echo "${item}" | cut -d':' -f2)
column=$(echo "${item}" | cut -d':' -f3)
NVIM_LISTEN_ADDRESS="/tmp/nvim-${pwd_escaped}" nvr -s --nostart --remote-send "<esc>:tab drop ${filename}<cr>${line}G${column}|"
```

3. Add a file called `vi` to your `$PATH`:
```bash
#!/bin/sh

pwd_escaped=$(pwd | sed 's!/!%!g')
NVIM_LISTEN_ADDRESS="/tmp/nvim-${pwd_escaped}" /usr/bin/nvim -p "$@"
```

4. `chmod +x open-in-nvim* vi`

5. Set `CARGO_OPEN=open-in-nvim-all` environment variable

6. Open two terminals
- run `cd to/your/project ; vi` in one of them
- run `cd to/your/project ; cargo lcheck` in the other

On any error or warning Neovim will open corresponding files in new or existing tabs and will set the cursor in each of them.
