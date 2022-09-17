# cargo-limit
[![Crates.io](https://img.shields.io/crates/v/cargo-limit.svg)](https://crates.io/crates/cargo-limit)

Cargo with less noise:
- errors have highest priority
    - they never appear in the middle of warnings
    - warnings are skipped by default until errors are fixed
    - external [path dependencies'](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#specifying-path-dependencies) warnings are skipped by default
- all messages come in reversed order by default
    - to avoid extra scrolling
- messages are grouped by filenames
- number of messages can be limited
- after encountering first error the rest of build time is limited by default
- files can be [automatically opened](EDITOR-INTEGRATION.md#neovim-integration) in your text editor on affected lines

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

## Neovim and other text editors/IDEs integration
See [here](EDITOR-INTEGRATION.md#neovim-integration).

## Environment variables
See [here](ENVIRONMENT-VARIABLES.md#environment-variables).

## Similar Projects
- [bacon](https://github.com/Canop/bacon) is a background rust code checker

## Thanks to all contributors ❤️
Thanks everyone for code contributions and bug reporting. Special thanks to [Casey Rodarmor](https://github.com/casey) for providing VimL code for quickfix populator and [Otavio Salvador](https://github.com/otavio) for NixOS package.

## Wanna contribute?
Please check out [issues](https://github.com/alopatindev/cargo-limit/issues) and [kanban board](https://github.com/alopatindev/cargo-limit/projects/1).

## License
MIT/Apache-2.0
