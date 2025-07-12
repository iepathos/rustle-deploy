use thiserror::Error;

#[derive(Debug, Error)]
pub enum InventoryError {
    #[error("Invalid YAML format: {reason}")]
    InvalidYaml { reason: String },

    #[error("Invalid JSON format: {reason}")]
    InvalidJson { reason: String },

    #[error("Invalid INI format: {reason}")]
    InvalidIni { reason: String },

    #[error("Unsupported inventory format")]
    UnsupportedFormat,

    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },

    #[error("Dynamic inventory script failed: {script}")]
    DynamicScriptFailed { script: String },

    #[error("Variable resolution failed: {variable}")]
    VariableResolution { variable: String },

    #[error("Host connectivity check failed: {host}")]
    ConnectivityFailed { host: String },

    #[error("Architecture detection failed: {host}")]
    ArchitectureDetectionFailed { host: String },

    #[error("Plan processing failed: {reason}")]
    PlanProcessingFailed { reason: String },

    #[error("Conversion error: {reason}")]
    ConversionError { reason: String },
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Duplicate host name: {host}")]
    DuplicateHost { host: String },

    #[error("Circular group dependency: {cycle:?}")]
    CircularGroupDependency { cycle: Vec<String> },

    #[error("Missing group: {group}")]
    MissingGroup { group: String },

    #[error("Invalid connection configuration for host: {host}")]
    InvalidConnection { host: String },

    #[error("Unreachable host: {host}")]
    UnreachableHost { host: String },
}

#[derive(Debug, Error)]
pub enum ProbeError {
    #[error("Connection failed: {host}")]
    ConnectionFailed { host: String },

    #[error("Authentication failed: {host}")]
    AuthenticationFailed { host: String },

    #[error("Command execution failed: {command}")]
    CommandFailed { command: String },

    #[error("Timeout exceeded: {timeout_secs}s")]
    Timeout { timeout_secs: u64 },
}

#[derive(Debug, Error)]
pub enum VariableError {
    #[error("Variable resolution failed: {variable}")]
    ResolutionFailed { variable: String },

    #[error("Circular variable dependency: {cycle:?}")]
    CircularDependency { cycle: Vec<String> },

    #[error("Invalid variable type: {variable}")]
    InvalidType { variable: String },

    #[error("Invalid host: {host}")]
    InvalidHost { host: String },

    #[error("Internal error: {message}")]
    InternalError { message: String },
}

#[derive(Debug, Error)]
pub enum DetectionError {
    #[error("Architecture detection failed: {reason}")]
    DetectionFailed { reason: String },

    #[error("Unsupported target: {target}")]
    UnsupportedTarget { target: String },

    #[error("Probe failed: {error}")]
    ProbeFailed { error: String },
}

#[derive(Debug, Error)]
pub enum ConversionError {
    #[error("Failed to convert to deployment target: {reason}")]
    ConversionFailed { reason: String },

    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Invalid configuration: {reason}")]
    InvalidConfiguration { reason: String },
}
