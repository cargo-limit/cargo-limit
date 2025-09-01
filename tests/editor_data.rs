use anyhow::{Context, Result};
use cargo_limit::{env_vars, models::EditorData, process::CARGO_EXECUTABLE};
use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, OnceLock},
};

const JQ_EXECUTABLE: &str = "jaq";
const JQ_VERSION: &str = "2.3.0";

#[test]
fn a() -> Result<()> {
    check("a")
}

#[ignore]
#[test]
fn b() -> Result<()> {
    check("b") // FIXME
}

#[ignore]
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
    let _ = fs::remove_dir_all(project_dir.join("target"));
    let target_dir = env::current_exe()?
        .parent()
        .context("parent")?
        .join("../../release");
    let bin_path = resolve_dependency(bin, &target_dir)?;
    let output = Command::new(bin_path)
        .args(args)
        .env(env_vars::EDITOR, resolve_jq(&target_dir)?)
        .env(env_vars::TIME_LIMIT, "0")
        .current_dir(&project_dir)
        .output()?;
    let data: EditorData = serde_json::from_slice(&output.stdout)?;

    assert_eq!(data.workspace_root, project_dir);
    assert!(!output.status.success() || data.locations.is_empty());
    if !output.status.success() {
        dbg!(&data);
    }

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

fn resolve_jq(target_dir: &Path) -> Result<PathBuf> {
    // it uses multiple temporary directories when called in parallel
    // which causes multiple unnecessary builds
    static MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    let mutex = MUTEX.get_or_init(|| Mutex::new(()));

    let bin_path = target_dir.join("bin").join(JQ_EXECUTABLE);
    {
        let _unused = mutex.lock();
        if !fs::exists(&bin_path)? {
            let output = Command::new(CARGO_EXECUTABLE)
                .args([
                    "install",
                    "--locked",
                    JQ_EXECUTABLE,
                    "--version",
                    JQ_VERSION,
                    "--root",
                    target_dir.to_str().context("target_dir")?,
                ])
                .output()?;
            assert!(output.status.success());
        }
    }
    Ok(bin_path)
}

fn resolve_dependency(bin: &str, target_dir: &Path) -> Result<PathBuf> {
    // file-locked by cargo, no need in mutex
    let bin_path = target_dir.join(bin);
    let output = Command::new(CARGO_EXECUTABLE)
        .args(["build", "--release", "--bin", bin])
        .output()?;
    assert!(output.status.success());
    Ok(bin_path)
}
