use anyhow::{Context, Result};
use cargo_limit::{env_vars, models::EditorData, process::CARGO_EXECUTABLE};
use cargo_metadata::diagnostic::DiagnosticLevel;
use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, OnceLock},
};

const JQ_EXECUTABLE: &str = "jaq";
const JQ_VERSION: &str = "2.3.0";

#[derive(Default)]
struct Warnings {
    force: bool,
    external_path_dependencies: bool,
}

#[test]
fn a() -> Result<()> {
    check("a")
}

#[test]
fn b() -> Result<()> {
    check("b")
}

#[test]
fn c() -> Result<()> {
    check("c")
}

#[test]
fn d() -> Result<()> {
    check_external_path_dependencies("d/d")
}

#[test]
fn e() -> Result<()> {
    check_external_path_dependencies("e/e")
}

fn check(project: &str) -> Result<()> {
    check_with("cargo-llcheck", &[], project, Warnings::default())?;
    check_with("cargo-lltest", &["--no-run"], project, Warnings::default())?;
    Ok(())
}

fn check_external_path_dependencies(project: &str) -> Result<()> {
    let few_messages = check_with(
        "cargo-llcheck",
        &[],
        project,
        Warnings {
            force: true,
            external_path_dependencies: false,
        },
    )?
    .locations;
    let more_messages = check_with(
        "cargo-llcheck",
        &[],
        project,
        Warnings {
            force: true,
            external_path_dependencies: true,
        },
    )?
    .locations;
    dbg!(project, few_messages.len(), more_messages.len());
    assert!(few_messages.len() < more_messages.len());
    Ok(())
}

fn check_with(bin: &str, args: &[&str], project: &str, warnings: Warnings) -> Result<EditorData> {
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
        .env(env_vars::FORCE_WARN, warnings.force.to_string().as_str())
        .env(
            env_vars::DEPS_WARN,
            warnings.external_path_dependencies.to_string().as_str(),
        )
        .current_dir(&project_dir)
        .output()?;
    let data: EditorData = serde_json::from_slice(&output.stdout)?;

    assert_eq!(data.workspace_root, project_dir);
    dbg!(&data);
    eprintln!("{}", String::from_utf8(output.stderr)?);

    let mut current_line = None;
    let mut current_path = None;
    let mut visited_paths = HashSet::<PathBuf>::default();
    let mut visited_warning = false;
    let mut visited_error = false;
    for i in &data.locations {
        if !visited_paths.contains(&i.path) {
            visited_paths.insert(i.path.clone());
            current_path = None;
            current_line = None;
            if !visited_warning {
                visited_warning = i.level == DiagnosticLevel::Warning;
            }
            if !visited_error {
                visited_error = i.level == DiagnosticLevel::Error;
            }
        }

        if i.level == DiagnosticLevel::Error {
            assert!(!visited_warning);
        } else if i.level == DiagnosticLevel::Warning && !warnings.external_path_dependencies {
            assert!(i.path.starts_with(&data.workspace_root));
        }

        if visited_warning {
            assert_eq!(i.level, DiagnosticLevel::Warning);
            if !warnings.force {
                assert!(!visited_error);
            }
        }

        if let Some(current_line) = current_line {
            assert!(i.line > current_line);
        }
        current_line = Some(i.line);

        if let Some(current_path) = current_path {
            assert_eq!(current_path, i.path);
        }
        current_path = Some(i.path.clone());
    }

    Ok(data)
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
