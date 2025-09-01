use anyhow::{Context, Result};
use cargo_limit::models::EditorData;
use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
fn a() -> Result<()> {
    check("a")
}

//#[ignore]
#[test]
fn b() -> Result<()> {
    check("b") // FIXME
}

//#[ignore]
#[test]
fn c() -> Result<()> {
    check("c") // FIXME
}

fn check(project: &str) -> Result<()> {
    check_with("cargo-llcheck", &[], project)?;
    check_with("cargo-lltest", &["--no-run"], project)?;
    Ok(())
}

fn check_with(bin: &str, args: &[&str], project: &str) -> Result<()> {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_dir = workspace_root.join("tests/stubs").join(project);
    let target_dir = env::current_exe()?
        .parent()
        .context("parent")?
        .join("../../release");
    let lltest = target_dir.join(bin);
    if !fs::exists(&lltest)? {
        assert!(
            Command::new("cargo")
                .args(["build", "--release", "--bin", bin])
                .output()?
                .status
                .success()
        );
    }
    let output = Command::new(lltest)
        .args(args)
        .env("CARGO_EDITOR", "xq")
        .current_dir(&project_dir)
        .output()?;
    assert!(!output.status.success());
    let data: EditorData = serde_json::from_slice(&output.stdout)?;

    dbg!(&data);
    assert_eq!(data.workspace_root, project_dir);
    assert!(!data.locations.is_empty());

    // TODO: distinguish warnings, normal errors and ICE errors?
    let mut current_line = None;
    let mut current_path = None;
    let mut visited_paths = HashSet::<PathBuf>::default();
    for i in data.locations {
        if !visited_paths.contains(&i.path) {
            visited_paths.insert(i.path.clone());
            current_line = Some(i.line);
            current_path = Some(i.path.clone());
        }
        if let Some(current_line) = current_line {
            assert!(i.line >= current_line);
        }
        assert_eq!(current_path, Some(i.path));
        current_line = Some(i.line);
    }

    Ok(())
}
