use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeployError {
    #[error("Compilation failed for target {target}: {reason}")]
    CompilationFailed { target: String, reason: String },

    #[error("Cross-compilation not supported for target: {target}")]
    UnsupportedTarget { target: String },

    #[error("Deployment failed to host {host}: {reason}")]
    DeploymentFailed { host: String, reason: String },

    #[error("Binary verification failed on {host}: expected {expected}, got {actual}")]
    VerificationFailed {
        host: String,
        expected: String,
        actual: String,
    },

    #[error("Module {module} not compatible with static linking")]
    StaticLinkingError { module: String },

    #[error("Binary size {size} exceeds limit {limit}")]
    BinarySizeExceeded { size: u64, limit: u64 },

    #[error("Deployment timeout exceeded: {timeout}s")]
    DeploymentTimeout { timeout: u64 },

    #[error("Rollback failed for deployment {deployment_id}: {reason}")]
    RollbackFailed {
        deployment_id: String,
        reason: String,
    },

    #[error("Cache corruption detected: {path}")]
    CacheCorruption { path: String },

    #[error("Insufficient disk space on {host}: required {required}, available {available}")]
    InsufficientSpace {
        host: String,
        required: u64,
        available: u64,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Template generation error: {0}")]
    TemplateGeneration(String),
}

pub type Result<T> = std::result::Result<T, DeployError>;
