use thiserror::Error;
use std::io;

#[derive(Error, Debug)]
pub enum ProcessError {
    #[error("Process failed to start: {0}")]
    StartError(String),

    #[error("Process terminated unexpectedly: {0}")]
    TerminationError(String),

    #[error("Output error: {0}")]
    OutputError(String),

    #[error("Invalid state: {0}")]
    StateError(String),

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Process not found")]
    ProcessNotFound,

    #[error("Process already exists")]
    ProcessExists,

    #[error("Invalid working directory: {0}")]
    InvalidWorkingDir(String),

    #[error("Failed to save process state: {0}")]
    StateSaveError(String),

    #[error("Failed to load process state: {0}")]
    StateLoadError(String),

    #[error("Failed to acquire lock: {0}")]
    LockError(String),

    #[error("Core error: {0}")]
    CoreError(#[from] blue_core::Error),
}

pub type Result<T> = std::result::Result<T, ProcessError>;
