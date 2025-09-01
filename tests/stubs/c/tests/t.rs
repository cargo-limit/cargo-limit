use c::models::EditorData;
use std::{collections::HashSet, path::PathBuf, process::Command};

#[test]
fn c() {
    let output = Command::new("ls").output().unwrap();
    let data: EditorData = serde_json::from_slice(&output.stdout).unwrap();
    let mut current = None;
    let mut visited = HashSet::<PathBuf>::default();
    for i in data.locations {
        if !visited.contains(&i.path) {
            visited.insert(i.path);
            current = Some(i.path);
        }
        assert_eq!(current, Some(i.path));
    }
}
