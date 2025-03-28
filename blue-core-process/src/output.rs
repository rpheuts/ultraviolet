use crate::error::{ProcessError, Result};
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use std::io::Write;

#[derive(Debug, Clone, Copy)]
pub enum Stream {
    Stdout,
    Stderr,
}

impl Stream {
    fn filename(&self) -> &'static str {
        match self {
            Stream::Stdout => "stdout.log",
            Stream::Stderr => "stderr.log",
        }
    }
}

pub struct OutputReader {
    output_dir: PathBuf,
}

impl OutputReader {
    pub fn new(output_dir: impl Into<PathBuf>) -> Self {
        Self {
            output_dir: output_dir.into(),
        }
    }

    pub async fn read_lines(&self, stream: Stream, lines: usize) -> Result<Vec<String>> {
        let path = self.output_dir.join(stream.filename());
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&path).await
            .map_err(|e| ProcessError::OutputError(format!("Failed to open {}: {}", path.display(), e)))?;
        
        let reader = BufReader::new(file);
        let mut lines_vec = Vec::new();
        let mut lines_iter = reader.lines();

        while let Some(line) = lines_iter.next_line().await
            .map_err(|e| ProcessError::OutputError(format!("Failed to read line: {}", e)))? {
            lines_vec.push(line);
        }

        // Keep only the last N lines
        if lines_vec.len() > lines {
            lines_vec = lines_vec.split_off(lines_vec.len() - lines);
            lines_vec.truncate(lines);  // Ensure we only keep exactly the number of lines requested
        }

        Ok(lines_vec)
    }

    pub async fn follow(&self, stream: Stream, writer: &mut dyn Write) -> Result<()> {
        let path = self.output_dir.join(stream.filename());
        if !path.exists() {
            return Ok(());
        }

        let file = File::open(&path).await
            .map_err(|e| ProcessError::OutputError(format!("Failed to open {}: {}", path.display(), e)))?;
        
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await
            .map_err(|e| ProcessError::OutputError(format!("Failed to read line: {}", e)))? {
            writeln!(writer, "{}", line)
                .map_err(|e| ProcessError::OutputError(format!("Failed to write line: {}", e)))?;
            writer.flush()
                .map_err(|e| ProcessError::OutputError(format!("Failed to flush output: {}", e)))?;
        }

        Ok(())
    }

    pub fn output_path(&self, stream: Stream) -> PathBuf {
        self.output_dir.join(stream.filename())
    }
}

pub struct OutputRotator {
    max_size: usize,
    max_files: usize,
}

impl OutputRotator {
    pub fn new(max_size: usize, max_files: usize) -> Self {
        Self {
            max_size,
            max_files,
        }
    }

    pub fn check_rotation(&self, path: impl AsRef<Path>) -> Result<bool> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(false);
        }

        let metadata = std::fs::metadata(path)
            .map_err(|e| ProcessError::OutputError(format!("Failed to get metadata for {}: {}", path.display(), e)))?;

        Ok(metadata.len() as usize >= self.max_size)
    }

    pub fn rotate(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(());
        }

        // Remove oldest file if it exists
        let max_backup = path.with_extension(format!("log.{}", self.max_files));
        if max_backup.exists() {
            std::fs::remove_file(&max_backup)
                .map_err(|e| ProcessError::OutputError(format!("Failed to remove old log: {}", e)))?;
        }

        // Rotate existing backups
        for i in (1..self.max_files).rev() {
            let src = path.with_extension(format!("log.{}", i));
            let dst = path.with_extension(format!("log.{}", i + 1));
            if src.exists() {
                std::fs::rename(&src, &dst)
                    .map_err(|e| ProcessError::OutputError(format!("Failed to rotate log: {}", e)))?;
            }
        }

        // Rotate current file
        if path.exists() {
            let backup = path.with_extension("log.1");
            std::fs::rename(path, backup)
                .map_err(|e| ProcessError::OutputError(format!("Failed to rotate current log: {}", e)))?;
        }

        Ok(())
    }
}
