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
    let bin = Path::new("src/bin");
    match fs::create_dir(bin) {
        Ok(_) => (),
        Err(err) if err.kind() == ErrorKind::AlreadyExists => (),
        err @ Err(_) => return err,
    }

    for i in SUBCOMMANDS {
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

    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
