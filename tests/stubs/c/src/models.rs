use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct A {
    pub b: Vec<B>,
}

#[derive(Deserialize)]
pub struct B {
    pub c: PathBuf,
}
