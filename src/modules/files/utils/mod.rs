//! Utility functions for file operations

pub mod atomic;
pub mod backup;
pub mod checksum;
pub mod ownership;
pub mod permissions;

pub use atomic::*;
pub use backup::*;
pub use checksum::*;
pub use ownership::*;
pub use permissions::*;

use thiserror::Error;

/// Common file operation errors
#[derive(Error, Debug)]
pub enum FileError {
    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },

    #[error("File not found: {path}")]
    NotFound { path: String },

    #[error("Invalid permissions format: {mode}")]
    InvalidPermissions { mode: String },

    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Template rendering failed: {source}")]
    TemplateError { source: handlebars::RenderError },

    #[error("IO error: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },

    #[error("JSON error: {source}")]
    Json {
        #[from]
        source: serde_json::Error,
    },
}
