use cargo_limit::models::EditorData;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process::Command,
};

// TODO: install xq or jaq?

fn check_editor_data(project: &str) -> anyhow::Result<()> {
    dbg!("check_editor_data 1");
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_dir = workspace_root.join("tests/stubs").join(project);
    dbg!("check_editor_data 2");
    let output = Command::new(workspace_root.join("target/release/cargo-llcheck"))
        .env("CARGO_EDITOR", "cat")
        .current_dir(&project_dir)
        .output()?;
    dbg!("check_editor_data 3");
    assert!(!output.status.success());
    let data: EditorData = serde_json::from_slice(&output.stdout)?;
    dbg!(&data); // TODO
    assert_eq!(data.workspace_root, project_dir);
    assert!(!data.locations.is_empty());

    dbg!("check_editor_data 4");
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
    dbg!("check_editor_data 5");
    Ok(())
}

#[test]
fn bugs() -> anyhow::Result<()> {
    check_editor_data("bugs")
}

#[test]
fn typos() -> anyhow::Result<()> {
    check_editor_data("typos")
}
