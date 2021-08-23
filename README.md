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
- files can be [automatically opened](NEOVIM-INTEGRATION.md) in your text editor on affected lines

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
Run any of these in your project:
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
- `CARGO_OPEN`
    - opens affected files in external application
        - see [neovim integration](NEOVIM-INTEGRATION.md) as example
    - empty (`""`) means don't run external application
    - empty is default
- `CARGO_OPEN_WARN`
    - open warnings (besides errors) in external application
    - `false` is default

## Why?
Initially it was just a workaround for [this issue](https://github.com/rust-lang/rust/issues/27189). Consider a program:
```rust
fn f() -> Result<(), ()> {
    Ok(())
}

fn main() {
    let mut i: u32 = 0;
    i -= 1;
    f();
    println!("Hello world");
}
```

It's counterproductive to read this kind of compiler noise in attempt to run the program:
```
$ cargo run
   Compiling hello v0.1.0 (/tmp/hello)
warning: variable `i` is assigned to, but never used
 --> src/main.rs:6:9
  |
6 |     let mut i: u32 = 0;
  |         ^^^^^
  |
  = note: `#[warn(unused_variables)]` on by default
  = note: consider using `_i` instead

warning: value assigned to `i` is never read
 --> src/main.rs:7:5
  |
7 |     i -= 1;
  |     ^
  |
  = note: `#[warn(unused_assignments)]` on by default
  = help: maybe it is overwritten before being read?

warning: unused `std::result::Result` that must be used
 --> src/main.rs:8:5
  |
8 |     f();
  |     ^^^^
  |
  = note: `#[warn(unused_must_use)]` on by default
  = note: this `Result` may be an `Err` variant, which should be handled

error: this arithmetic operation will overflow
 --> src/main.rs:7:5
  |
7 |     i -= 1;
  |     ^^^^^^ attempt to compute `0_u32 - 1_u32` which would overflow
  |
  = note: `#[deny(arithmetic_overflow)]` on by default

error: aborting due to previous error; 3 warnings emitted

error: could not compile `hello`.

To learn more, run the command again with --verbose.
```

All we want on this development iteration is to focus on this error:
```
$ cargo lrun
   Compiling hello v0.1.0 (/tmp/hello)
error: this arithmetic operation will overflow
 --> src/main.rs:7:5
  |
7 |     i -= 1;
  |     ^^^^^^ attempt to compute `0_u32 - 1_u32` which would overflow
  |
  = note: `#[deny(arithmetic_overflow)]` on by default

error: could not compile `hello`.

To learn more, run the command again with --verbose.
```

After fixing it we probably want to see the first warning(s):
```
$ sed -i '/.*i -= 1;/d' src/main.rs
$ CARGO_MSG_LIMIT=1 cargo lrun
    Finished dev [unoptimized + debuginfo] target(s) in 0.00s
warning: unused variable: `i`
 --> src/main.rs:6:9
  |
6 |     let mut i: u32 = 0;
  |         ^^^^^ help: if this is intentional, prefix it with an underscore: `_i`
  |
  = note: `#[warn(unused_variables)]` on by default

     Running `target/debug/hello`
Hello world
```

## License
MIT/Apache-2.0
