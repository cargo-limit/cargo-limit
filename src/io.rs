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
        FlushingWriter { writer }
    }
}

impl<W: Write> Write for FlushingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written = self.writer.write(buf)?;
        self.flush()?;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl Buffers {
    pub fn new(child: &mut Child) -> Result<Self> {
        let child_stdout_reader =
            io::BufReader::new(child.stdout.take().context("cannot read stdout")?);
        let stdout_writer = FlushingWriter::new(io::stdout());
        let stderr_writer = FlushingWriter::new(io::stderr());
        Ok(Self {
            child_stdout_reader,
            stdout_writer,
            stderr_writer,
        })
    }

    pub fn map_child_stdout_reader<'this, T>(
        &'this mut self,
        f: impl FnOnce(&'this mut io::BufReader<ChildStdout>) -> T,
    ) -> T {
        f(&mut self.child_stdout_reader)
    }

    pub fn write_to_stdout(&mut self, text: &str) -> io::Result<()> {
        std::write!(&mut self.stdout_writer, "{}", text)
    }

    pub fn writeln_to_stdout(&mut self, text: &str) -> io::Result<()> {
        std::writeln!(&mut self.stdout_writer, "{}", text)
    }

    pub fn write_to_stderr(&mut self, text: String) -> io::Result<()> {
        std::write!(&mut self.stderr_writer, "{}", text)
    }

    pub fn write_all_to_stderr(&mut self, buf: &[u8]) -> io::Result<()> {
        self.stderr_writer.write_all(buf)
    }

    pub fn copy_from_child_stdout_reader_to_stdout_writer(&mut self) -> io::Result<u64> {
        io::copy(&mut self.child_stdout_reader, &mut self.stdout_writer)
    }
}
