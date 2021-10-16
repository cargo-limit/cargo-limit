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

In order to not disrupt from text editing or file navigation, this will work only if
- current mode is normal
- current buffer is either empty or contains some existing and unmodified (saved) file.

## Custom open handler
If you want something different than opening/switching tabs with affected files — you can add your own handler to `init.vim`.

If you want to open files **in buffers instead of tabs** — try this:
```viml
function! g:CargoLimitOpen(editor_data)
  let l:initial_file = resolve(expand('%:p'))
  if l:initial_file != '' && !filereadable(l:initial_file)
    return
  endif
  for source_file in reverse(a:editor_data.files)
    let l:path = fnameescape(source_file.path)
    if mode() == 'n' && &l:modified == 0
      execute 'edit ' . l:path
      call cursor((source_file.line), (source_file.column))
    else
      break
    endif
  endfor
endfunction
```

If you want to populate a **quickfix list** — try that:
```viml
set errorformat =%f:%l:%c:%m

function! g:CargoLimitOpen(editor_data)
  let l:winnr = winnr()

  cgetexpr []
  for file in a:editor_data['files']
    caddexpr file['path'] . ':' . file['line'] . ':' . file['column'] . ':' . file['message']
  endfor

  if empty(a:editor_data['files'])
    cclose
  else
    copen
  endif

  if l:winnr !=# winnr()
    wincmd p
  endif
endfunction
```

# Other text editors/IDEs integration
cargo-limit can run external app/script and provide affected files to stdin in the following JSON format:
```json
{
  "workspace_root": "/full/path/to/project",
  "files": [
    {
      "path": "/full/path/to/project/file.rs",
      "line": 4,
      "column": 1,
      "message": "unused import: `diagnostic::DiagnosticSpan`",
      "level": "warning"
    }
  ]
}
```

Theoretically this can be used for any text editor or IDE, especially if it supports client/server communication. To do that you need a wrapper app/script that parses the `files` and gives them to the text editor or IDE client.

## Example: gedit
1. Install [`jq`](https://stedolan.github.io/jq/download/)
2. Create `open-in-gedit.sh`:
```bash
#!/bin/bash

jq --raw-output '. as $root | $root | .files[] | [
    "gedit",
    .path,
    "+" + (.line | tostring) + ":" + (.column | tostring),
    "&"
] | join(" ")' | bash
```
3. `chmod +x open-in-gedit.sh`
4. Set `CARGO_EDITOR=/path/to/open-in-gedit.sh` environment variable
5. Run `cargo lrun` in your project directory
6. In case of compiling errors `open-in-gedit.sh` will open files in `gedit` on affected lines and columns
7. `cargo llrun` (`cargo llcheck`, etc.) will open them in case of warnings as well.
