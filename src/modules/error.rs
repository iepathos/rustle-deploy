//! Error types for execution modules

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModuleExecutionError {
    #[error("Module not found: {0}")]
    ModuleNotFound(String),

    #[error("Invalid arguments: {0}")]
    InvalidArgs(String),

    #[error("Platform not supported: {0:?}")]
    UnsupportedPlatform(crate::modules::Platform),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Command parsing error: {0}")]
    CommandParsingError(#[from] shell_words::ParseError),

    #[error("Validation error: {0}")]
    ValidationError(#[from] ValidationError),
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Missing required argument: {0}")]
    MissingRequiredArg(String),

    #[error("Invalid argument type: {0}")]
    InvalidArgType(String),

    #[error("Invalid argument value: {0}")]
    InvalidArgValue(String),
}

#[derive(Debug, Error)]
pub enum PackageManagerError {
    #[error("Package manager not available: {0}")]
    NotAvailable(String),

    #[error("Package not found: {0}")]
    PackageNotFound(String),

    #[error("Operation failed: {0}")]
    OperationFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum ServiceManagerError {
    #[error("Service manager not available: {0}")]
    NotAvailable(String),

    #[error("Service not found: {0}")]
    ServiceNotFound(String),

    #[error("Operation failed: {0}")]
    OperationFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
