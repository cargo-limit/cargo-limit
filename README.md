# cargo-limit
[![Crates.io](https://img.shields.io/crates/v/cargo-limit.svg)](https://crates.io/crates/cargo-limit)

Cargo with less noise:
- errors have highest priority
    - they never appear in the middle of warnings
    - warnings are skipped by default until errors are fixed
    - all dependencies' warnings are skipped by default
- all messages come in reversed order by default
    - to avoid extra scrolling
- [duplicated messages](https://github.com/rust-lang/cargo/issues/3531#issuecomment-272043238) are skipped
- messages are grouped by filenames
- number of messages can be limited
- after encountering first error the rest of build time is limited by default
- files can be [automatically opened](#neovim-integration) in your text editor on affected lines

This tool is especially useful in combination with [cargo-watch](https://crates.io/crates/cargo-watch).

[![asciicast](https://asciinema.org/a/372235.svg)](https://asciinema.org/a/372235)

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

### Environment variables
- `CARGO_MSG_LIMIT`
    - limit compiler messages number
    - `0` means no limit, which is default
- `CARGO_TIME_LIMIT`
    - `cargo` execution time limit in seconds after encountering first compiling error
    - `1` is default
    - `0` means no limit
- `CARGO_ASC`
    - show compiler messages in ascending order
    - `false` is default
- `CARGO_FORCE_WARN`
    - show warnings even if errors still exist
    - `false` is default
- `CARGO_DEPS_WARN`
    - show dependencies' warnings
    - `false` is default
- `CARGO_EDITOR`
    - opens affected files in external app
        - see [neovim integration](NEOVIM-INTEGRATION.md) as example
    - empty (`""`) means don't run external app
    - `"_cargo-limit-open-in-nvim"` is default

## Neovim integration
Enable the plugin in your `~/.config/nvim/init.vim`. For instance for [vim-plug](https://github.com/junegunn/vim-plug#neovim):
```viml
Plug 'alopatindev/cargo-limit', { 'branch': 'nvim-plugin', 'do': 'cargo install --force --git https://github.com/alopatindev/nvim-send' }
```
and install it with `nvim +PlugInstall +UpdateRemotePlugins +qa`

### Testing
1. Open two terminals
2. `cd your/project/directory` in both of them
3. Run `nvim` in one of them
4. Run `cargo lrun` in the other
5. In case of compiling error `nvim` will open new or existing tab with the file on affected line
6. Use `cargo llrun` (`llcheck`, etc.) to make Neovim react on warnings besides errors as well.

### Other text editors/IDEs integration
TODO

## Why "limit"?
Initially it was just a workaround for [this issue](https://github.com/rust-lang/rust/issues/27189).

## License
MIT/Apache-2.0
