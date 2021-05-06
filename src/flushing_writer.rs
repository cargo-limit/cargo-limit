use std::io::{self, Write};

#[derive(Clone)]
pub struct FlushingWriter<W> {
    writer: W,
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
