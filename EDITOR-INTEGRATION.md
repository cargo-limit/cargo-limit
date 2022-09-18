# Neovim Integration
Enable the plugin in your `init.vim`. For instance for [vim-plug](https://github.com/junegunn/vim-plug#neovim):
```viml
Plug 'alopatindev/cargo-limit', { 'do': 'cargo install cargo-limit nvim-send' }
```
and install it with `nvim +PlugInstall +UpdateRemotePlugins +qa`

## Testing
1. Open two terminals
2. `cd your/project/directory` in both of them
3. Run `nvim` in one of them
4. Run `cargo lrun` in the other
5. In case of compiling errors `nvim` opens new or existing tab with the file on affected line and column
6. Fix the error, save the file and `nvim` will jump to the next error location
7. `cargo llrun` (`cargo llcheck`, etc.) will open them in case of warnings as well.

In order to not disrupt from text editing or file navigation, this will work only if
- current mode is normal
- current buffer is either empty or contains some existing and unmodified (saved) file.

## Customizations
⚠️ If you want other behavior than opening/switching tabs with affected files — you can add your custom open handler to `init.vim`.

<details>
<summary>💡 Examples</summary>
<p>

### Open Files in Buffers Instead of Tabs
```viml
function! g:CargoLimitOpen(editor_data)
  let l:current_file = resolve(expand('%:p'))
  if l:current_file != '' && !filereadable(l:current_file)
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

### Populate a QuickFix List
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

</p>
</details>

# Other Text Editors/IDEs Integration
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

jq --raw-output '.files |= unique_by(.path) | .files[] | [
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
