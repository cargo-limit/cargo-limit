use c::models::A;
use std::{collections::HashSet, path::PathBuf, process::Command};

#[test]
fn c() {
    let output = Command::new("").output().unwrap();
    let data: A = serde_json::from_slice(&output.stdout).unwrap();
    let mut current = None;
    let mut visited = HashSet::<PathBuf>::default();
    for i in data.b {
        if !visited.contains(&i.c) {
            visited.insert(i.c);
            current = Some(i.c);
        }
        assert_eq!(current, Some(i.c));
    }
}
