use std::{
    fs,
    io::{Error, ErrorKind, Write},
    path::Path,
};

const BODY: &str = "fn main() -> anyhow::Result<()> { cargo_limit::run_subcommand() }";
const SUBCOMMANDS: &[&str] = &[
    "bench", "build", "check", "clippy", "doc", "fix", "run", "rustc", "rustdoc", "test",
];

fn main() -> Result<(), Error> {
    println!("cargo:rerun-if-changed=build.rs");

    let bin = Path::new("src/bin");
    match fs::create_dir(bin) {
        Ok(_) => (),
        Err(err) if err.kind() == ErrorKind::AlreadyExists => (),
        err @ Err(_) => return err,
    }

    let subcommands = SUBCOMMANDS
        .iter()
        .map(|i| i.to_string())
        .chain(SUBCOMMANDS.iter().map(|i| format!("l{}", i)));

    for i in subcommands {
        let file = bin.join(format!("cargo-l{}.rs", i));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(file)
        {
            Ok(mut file) => {
                file.write_all(BODY.as_bytes())?;
            },
            Err(err) if err.kind() == ErrorKind::AlreadyExists => (),
            Err(err) => return Err(err),
        };
    }

    Ok(())
}
