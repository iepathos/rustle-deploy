use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OutputError {
    #[error("Source binary not found: {path}")]
    SourceNotFound { path: PathBuf },

    #[error("Incompatible source type for this strategy")]
    IncompatibleSource,

    #[error("No compatible output strategy available")]
    NoCompatibleStrategy,

    #[error("Copy verification failed: expected {expected} bytes, got {actual}")]
    VerificationFailed { expected: u64, actual: u64 },

    #[error("Copy failed from {source_path} to {destination}: {message}")]
    CopyFailed {
        source_path: String,
        destination: PathBuf,
        message: String,
    },

    #[error("Insufficient disk space: need {needed} bytes, available {available}")]
    InsufficientSpace { needed: u64, available: u64 },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
