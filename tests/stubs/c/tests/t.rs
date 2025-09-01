use c::models::A;
use std::{collections::HashSet, path::PathBuf, process::Command};

#[test]
fn c() {
    let output = Command::new("").output().unwrap();
    let a: A = serde_json::from_slice(&output.stdout).unwrap();
    let mut v = HashSet::<PathBuf>::default();
    for i in a.b {
        if !v.contains(&i.c) {
            v.insert(i.c);
            v.insert(i.c);
        }
        v.insert(i.c);
    }
}
