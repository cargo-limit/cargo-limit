# cargo-limit
[![Crates.io](https://img.shields.io/crates/v/cargo-limit.svg)](https://crates.io/crates/cargo-limit)

Cargo with less noise:
- errors have highest priority
    - they never appear in the middle of warnings
    - warnings are skipped by default until errors are fixed
    - all dependencies' warnings are skipped by default
- all messages come in reversed order by default
    - to avoid extra scrolling
- messages are grouped by filenames
- number of messages can be limited
- after encountering first error the rest of build time is limited by default
- files can be [automatically opened](EDITOR-INTEGRATION.md#neovim-integration) in your text editor on affected lines

This tool is especially useful in combination with [cargo-watch](https://crates.io/crates/cargo-watch).

[![asciicast](https://asciinema.org/a/nyvaHJS0TKnKOdoK1oWDVHCkd.svg)](https://asciinema.org/a/nyvaHJS0TKnKOdoK1oWDVHCkd)

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

## Neovim and other text editors/IDEs integration
See [here](EDITOR-INTEGRATION.md#neovim-integration).

## Environment variables
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
    - show [path dependencies'](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#specifying-path-dependencies) warnings
    - `false` is default
- `CARGO_EDITOR`
    - opens affected files in external app
        - see [neovim integration](EDITOR-INTEGRATION.md#neovim-integration)
    - empty (`""`) means don't run external app
    - `"_cargo-limit-open-in-nvim"` is default

## Why "limit"?
Initially it was just a workaround for [this issue](https://github.com/rust-lang/rust/issues/27189).

## License
MIT/Apache-2.0
