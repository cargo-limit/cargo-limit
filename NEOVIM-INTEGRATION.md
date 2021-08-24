# Neovim integration
`cargo-limit` can run an external app, providing it a list of affected files, lines and columns as arguments in the following format:

```
/full/path/to/project relative/path/to/file1.rs:10:5 relative/path/to/file2.rs:4:1
```

TODO: maybe send json instead? these args don't work with weird characters and/or spaces
TODO: use stdin instead of args?

Theoretically this can be used for any text editor or IDE which supports client/server communication. In order to do that you need a wrapper script that parses the list and gives it to the text editor or IDE client.

## Installation
1. Add to your `~/.config/nvim/init.vim`:
```viml
"TODO: use s: in plugin
"TODO: detect OS, set named pipe on windows
"TODO: socket permissions?
"TODO: escape paths with spaces and weird characters?
"TODO: escape windows username?
"TODO: detect whether cargo installed?
function! s:on_cargo_metadata_stdout(job_id, data, _event)
  let l:stdout = join(a:data, '')
  if len(l:stdout) > 0
    let l:metadata = json_decode(l:stdout)
    let l:workspace_root = get(l:metadata, 'workspace_root')
    let l:escaped_workspace_root = substitute(workspace_root, '/', '%', 'g')
    let l:socket_path = '/tmp/nvim-cargo-limit-' . $USER . '-' . l:escaped_workspace_root
    if !filereadable(l:socket_path)
      call serverstart(l:socket_path)
    endif
  endif
endfunction

if has('nvim')
  call jobstart(['cargo', 'metadata', '--quiet', '--format-version=1'], {
  \ 'on_stdout': function('s:on_cargo_metadata_stdout')
  \ })
else
  throw 'unsupported text editor'
endif
```

2. Run `pip3 install --user neovim-remote` and check that `nvr --version` runs without errors (`$HOME/.local/bin` should be listed in your `$PATH`) (TODO: remove)

3. Put a file called `open-in-nvim` somewhere in your `$PATH`:
```bash
#!/bin/bash

workspace_root="$1"
workspace_root_escaped=$(echo "$1" | sed 's!/!%!g')
nvim_listen_address="/tmp/nvim-cargo-limit-${USER}-${workspace_root_escaped}"

shift
files=( "$@" )
cmd=''
for ((i=${#files[@]}-1; i>=0; i--)); do
    item="${files[$i]}"
    filename=$(echo "${item}" | cut -d':' -f1)
    filename=$(printf "${workspace_root}/%q" "${filename}")
    line=$(echo "${item}" | cut -d':' -f2)
    column=$(echo "${item}" | cut -d':' -f3)
    cmd+="<esc>:tab drop ${filename}<cr>${line}G${column}|"
done

nvr -s --nostart --servername "${nvim_listen_address}" --remote-send "${cmd}"
```

4. `chmod +x open-in-nvim`

5. Set `CARGO_OPEN=open-in-nvim` environment variable

6. Open two terminals
- run `cd to/your/project ; vi` in one of them
- run `cd to/your/project ; cargo lcheck` in the other

TODO: cross-platform commands

TODO: use subtitles instead of just list with numbers

## Result
For each file affected by error (or warning as well, in case of running `cargo llcheck`) Neovim will:
- open it in new or existing tab
- jump to the corresponding line and column
