/*use anyhow::{Context, Result};
use std::{
    io::{self, Write},
    process::{Child, ChildStdout},
};

#[derive(Clone)]
pub struct FlushingWriter<W> {
    writer: W,
}

pub struct Buffers {
    child_stdout_reader: io::BufReader<ChildStdout>,
    stdout_writer: FlushingWriter<io::Stdout>,
    stderr_writer: FlushingWriter<io::Stderr>,
}*/
