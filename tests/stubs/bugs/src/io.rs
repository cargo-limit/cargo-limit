use anyhow::{Context, Result};
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
}

impl<W> FlushingWriter<W> {
    pub fn new(writer: W) -> Self {
        todo!()
    }
}

impl<W: Write> Write for FlushingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        todo!()
    }

    fn flush(&mut self) -> io::Result<()> {
        todo!()
    }
}

impl Buffers {
    pub fn new(child: &mut Child) -> Result<Self> {
        todo!()
    }

    pub fn map_child_stdout_reader<'this, T>(
        &'this mut self,
        f: impl FnOnce(&'this mut io::BufReader<ChildStdout>) -> T,
    ) -> T {
        todo!()
    }

    pub fn write_to_stdout(&mut self, text: &str) -> io::Result<()> {
        todo!()
    }

    pub fn writeln_to_stdout(&mut self, text: &str) -> io::Result<()> {
        todo!()
    }

    pub fn write_to_stderr(&mut self, text: String) -> io::Result<()> {
        todo!()
    }

    pub fn write_all_to_stderr(&mut self, buf: &[u8]) -> io::Result<()> {
        todo!()
    }

    pub fn copy_from_child_stdout_reader_to_stdout_writer(&mut self) -> io::Result<u64> {
        todo!()
    }
}
