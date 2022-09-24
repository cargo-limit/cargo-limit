# cargo-limit
[![Crates.io](https://img.shields.io/crates/v/cargo-limit.svg)](https://crates.io/crates/cargo-limit)

🚀 Cargo with less noise:
- errors have highest priority
    - they never appear in the middle of warnings
    - **warnings are skipped** by default until errors are fixed
    - external [path dependencies'](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#specifying-path-dependencies) warnings are skipped by default
- all messages come **in reversed order** by default
    - to avoid extra scrolling
- messages are grouped by filenames
- number of messages can be limited
- after encountering **first error** the rest of **build time is limited** by default
- files can be **[automatically opened](#text-editoride-integrations) in your text editor on affected lines**

This tool is especially useful in combination with [cargo-watch](https://crates.io/crates/cargo-watch).

Initially this project was just a workaround for [this issue](https://github.com/rust-lang/rust/issues/27189).

[![asciicast](https://asciinema.org/a/441673.svg)](https://asciinema.org/a/441673)

## Installation

### From crates.io
```
cargo install cargo-limit
```

### From git
```
cargo install --force --git https://github.com/alopatindev/cargo-limit
```

## Usage
Run any of these in your project directory:
```
cargo lbench
cargo lbuild
cargo lcheck
cargo lclippy
cargo ldoc
cargo lfix
cargo lrun
cargo lrustc
cargo lrustdoc
cargo ltest
```

Also `llcheck`, `llrun`, etc.

<details>
<summary><b>💡 Environment Variables 👁️</b></summary>
<p>

### CARGO_MSG_LIMIT
- limit compiler messages number
- `0` means no limit, which is default

### CARGO_TIME_LIMIT
- `cargo` execution time limit in seconds after encountering first compiling error
- `1` is default
- `0` means no limit

### CARGO_ASC
- show compiler messages in ascending order
- `false` is default

### CARGO_FORCE_WARN
- show warnings even if errors still exist
- `false` is default

### CARGO_DEPS_WARN
- show external path dependencies' warnings
- `false` is default

### CARGO_EDITOR
- opens affected files in external app
    - see [neovim integration](#text-editoride-integrations)
- empty (`""`) means don't run external app
- `"_cargo-limit-open-in-nvim"` is default

</p>
</details>

## Text Editor/IDE integrations
<details>
<summary><b>💡 Neovim Plugin 👁️</b></summary>
<p>

Enable the plugin in your `init.vim`. For instance for [vim-plug](https://github.com/junegunn/vim-plug#neovim):
```viml
Plug 'alopatindev/cargo-limit', { 'do': 'cargo install cargo-limit nvim-send' }

"" Optionally: F2 to save, if already saved - jump to next location
"function g:SaveAllOrOpenNextLocation()
"  if &l:modified == 0
"    call g:CargoLimitOpenNextLocation()
"  else
"    execute 'wa'
"  endif
"endfunction
"
"nmap <F2> :call g:SaveAllOrOpenNextLocation()<cr>
"vmap <F2> <esc>:call g:SaveAllOrOpenNextLocation()<cr>v
"imap <F2> <esc>:call g:SaveAllOrOpenNextLocation()<cr>i
```
and install it with

```bash
nvim +PlugInstall +UpdateRemotePlugins +qa
```

<details>
<summary><b>💡 Test the plugin! 👁️</b></summary>
<p>

1. Open two terminals (or tmux windows, etc.)
2. `cd your/project/directory` in both of them
3. Run `nvim` in one of them
4. Run `cargo lrun` in the other
5. In case of compiling errors `nvim` opens new or existing tabs with the files on affected lines and columns
6. Fix the error, save the file and `nvim` will jump to the next error location
7. `cargo llrun` (`cargo llcheck`, etc.) will open them in case of warnings as well.

</p>
</details>

<details>
<summary><b>⚠️ Known Limitations! 👁️</b></summary>
<p>

### 1. Auto-jumps work only if
- current **mode is normal**
- current buffer is either **empty or contains some existing and unmodified** (saved) file

This is by design, in order to **not disrupt** from active text editing or file navigation process.

### 2. Auto-jump on each file save is currently imprecise
- it may jump to a wrong line if it moved
- it may not jump at all, if the next affected line is supposed to be modified already

For precise jump please rerun `cargo ll{check,run,etc.}`.

### 3. Before running `nvim`: Current Directory should be Project (sub)directory
- that's required so **cargo-limit** could [figure out](https://github.com/alopatindev/cargo-limit/issues/30#issuecomment-1219793195) which exact `nvim` instance should be controlled
- only **first `nvim` instance** with current project (sub)directory will be **controlled by cargo-limit**.

</p>
</details>

## Customizations
Add a **custom open handler** to your `init.vim` if you want other Neovim behavior.

<details>
<summary><b>💡 See examples! 👁️</b></summary>
<p>

### Open Files in Buffers Instead of Tabs
```viml
function! g:CargoLimitOpen(editor_data)
  let l:current_file = resolve(expand('%:p'))
  if l:current_file != '' && !filereadable(l:current_file)
    return
  endif
  for location in reverse(a:editor_data.locations)
    let l:path = fnameescape(location.path)
    if mode() == 'n' && &l:modified == 0
      execute 'edit ' . l:path
      call cursor((location.line), (location.column))
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
  for file in a:editor_data['locations']
    caddexpr file['path'] . ':' . file['line'] . ':' . file['column'] . ':' . file['message']
  endfor

  if empty(a:editor_data['locations'])
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

</p>
</details>

<details>
<summary><b>💡 Other Text Editors/IDEs 👁️</b></summary>
<p>

**cargo-limit** can run external app/script and provide affected locations to stdin in the following JSON format:
```json
{
  "workspace_root": "/full/path/to/project",
  "locations": [
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

Theoretically this can be used for any text editor or IDE, especially if it supports client/server communication. To do that you need a **wrapper app/script** that parses the `locations` and gives them to the text editor or IDE client.

<details>
<summary><b>💡 Example: Gedit! 👁️</b></summary>
<p>

1. Install [`jq`](https://stedolan.github.io/jq/download/)
2. Create `open-in-gedit.sh`:
```bash
#!/bin/bash

jq --raw-output '.locations |= unique_by(.path) | .locations[] | [
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

</p>
</details>

</p>
</details>

## Similar Projects
[bacon](https://github.com/Canop/bacon) is a background rust code checker.

## Thanks to all Contributors ❤️
Thanks everyone for code contributions and bug reporting. Special thanks to [Casey Rodarmor](https://github.com/casey) for providing VimL code for quickfix populator and [Otavio Salvador](https://github.com/otavio) for NixOS package.

## Wanna Contribute?
Please check out [issues](https://github.com/alopatindev/cargo-limit/issues) and [kanban board](https://github.com/alopatindev/cargo-limit/projects/1?fullscreen=true).

## License
MIT/Apache-2.0
