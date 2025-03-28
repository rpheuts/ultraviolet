use thiserror::Error;
use std::io;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Module error: {0}")]
    Module(String),

    #[error("Invalid manifest: {0}")]
    Manifest(String),

    #[error("Schema validation error: {0}")]
    Schema(String),

    #[error("Method not found: {0}")]
    MethodNotFound(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
