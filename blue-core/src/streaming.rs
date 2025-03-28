use std::io::{Write, BufWriter};
use serde::Serialize;
use crate::error::Result;

pub struct StreamWriter<W: Write> {
    inner: BufWriter<W>,
}

impl<W: Write> StreamWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            inner: BufWriter::new(writer)
        }
    }

    // Basic write for raw content (tokens, chunks, etc)
    pub fn write(&mut self, content: &str) -> Result<()> {
        self.inner.write_all(content.as_bytes())?;
        self.inner.flush()?;
        Ok(())
    }

    // Convenience method for line-oriented content
    pub fn write_line(&mut self, line: &str) -> Result<()> {
        self.write(line)?;
        self.write("\n")
    }

    pub fn write_json<T: Serialize>(&mut self, value: &T) -> Result<()> {
        serde_json::to_writer(&mut self.inner, value)?;
        self.inner.flush()?;
        Ok(())
    }
}
