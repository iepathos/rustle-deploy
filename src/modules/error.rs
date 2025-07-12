use anyhow;
use thiserror::Error;

/// Errors that can occur during module operations
#[derive(Error, Debug)]
pub enum ModuleError {
    #[error("Module not found: {name} (searched: {searched_sources:?})")]
    ModuleNotFound {
        name: String,
        searched_sources: Vec<String>,
    },

    #[error("Unsupported module source: {0}")]
    UnsupportedSource(String),

    #[error("Failed to load module from {location}: {error}")]
    LoadError { location: String, error: String },

    #[error("Module compilation failed: {error}")]
    CompilationError { error: String },

    #[error("Dependency not found: {name} version {version_req}")]
    DependencyNotFound { name: String, version_req: String },

    #[error("Module validation failed: {errors:?}")]
    ValidationFailed { errors: Vec<String> },

    #[error("Security violation in module execution: {operation} - {reason}")]
    SecurityViolation { operation: String, reason: String },

    #[error("Invalid module argument: {arg} = {value}")]
    InvalidArg { arg: String, value: String },

    #[error("Invalid arguments: {message}")]
    InvalidArgs { message: String },

    #[error("Unsupported platform: {0}")]
    UnsupportedPlatform(crate::modules::interface::Platform),

    #[error("Module execution failed: {message}")]
    ExecutionFailed { message: String },

    #[error("Cache error: {0}")]
    Cache(#[from] CacheError),

    #[error("Resolve error: {0}")]
    Resolve(#[from] ResolveError),

    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("Compile error: {0}")]
    Compile(#[from] CompileError),
}

/// Errors that can occur during module source resolution
#[derive(Error, Debug)]
pub enum ResolveError {
    #[error("Incompatible source type")]
    IncompatibleSource,

    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Invalid module: {reason}")]
    InvalidModule { reason: String },

    #[error("I/O error during {operation}: {error}")]
    IoError { operation: String, error: String },

    #[error("Not found: {name}")]
    NotFound { name: String },

    #[error("Git error during {operation}: {error}")]
    GitError { operation: String, error: String },

    #[error("HTTP error for {url}: {error}")]
    HttpError { url: String, error: String },

    #[error("Registry error for {registry}: {error}")]
    RegistryError { registry: String, error: String },

    #[error("Unknown registry: {name}")]
    UnknownRegistry { name: String },
}

/// Errors that can occur during module validation
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Validation failed with errors: {errors:?} and warnings: {warnings:?}")]
    Failed {
        errors: Vec<String>,
        warnings: Vec<String>,
    },

    #[error("Security policy violation: {policy} - {reason}")]
    SecurityPolicyViolation { policy: String, reason: String },

    #[error("Compatibility check failed: {requirement} not met")]
    CompatibilityError { requirement: String },

    #[error("Invalid dependency: {dependency} - {reason}")]
    InvalidDependency { dependency: String, reason: String },

    #[error("Circular dependency detected: {chain:?}")]
    CircularDependency { chain: Vec<String> },

    #[error("Missing required argument: {arg}")]
    MissingRequiredArg { arg: String },

    #[error("Invalid argument value: {arg} = {value} - {reason}")]
    InvalidArgValue {
        arg: String,
        value: String,
        reason: String,
    },
}

/// Errors that can occur during module compilation
#[derive(Error, Debug)]
pub enum CompileError {
    #[error("Template error in {template}: {error}")]
    TemplateError { template: String, error: String },

    #[error("Code generation failed: {reason}")]
    CodeGenerationFailed { reason: String },

    #[error("Static data compilation failed: {error}")]
    StaticDataError { error: String },

    #[error("Module wrapper generation failed: {error}")]
    WrapperGenerationFailed { error: String },

    #[error("Rust compilation failed: {output}")]
    RustCompilationFailed { output: String },

    #[error("Syntax error: {message}")]
    SyntaxError { message: String },

    #[error("Validation failed: {reason}")]
    ValidationFailed { reason: String },

    #[error("Cross-compilation failed for target {target}: {error}")]
    CrossCompilationFailed { target: String, error: String },
}

/// Errors that can occur during code generation
#[derive(Error, Debug)]
pub enum GenerationError {
    #[error("Template not found: {template}")]
    TemplateNotFound { template: String },

    #[error("Template rendering failed: {error}")]
    TemplateRenderError { error: String },

    #[error("Invalid template context: {field} is missing")]
    InvalidContext { field: String },

    #[error("Output generation failed: {error}")]
    OutputError { error: String },
}

/// Errors that can occur during module caching
#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Cache entry not found: {key}")]
    NotFound { key: String },

    #[error("Cache entry expired: {key}")]
    Expired { key: String },

    #[error("I/O error during {operation}: {error}")]
    IoError { operation: String, error: String },

    #[error("Serialization error during {operation}: {error}")]
    SerializationError { operation: String, error: String },

    #[error("Cache corruption detected: {reason}")]
    Corruption { reason: String },

    #[error("Cache size limit exceeded: {current_size} > {max_size}")]
    SizeLimitExceeded {
        current_size: usize,
        max_size: usize,
    },
}

/// Errors that can occur in the dependency resolution system
#[derive(Error, Debug)]
pub enum DependencyError {
    #[error("Version conflict: {package} requires {conflicting_versions:?}")]
    VersionConflict {
        package: String,
        conflicting_versions: Vec<String>,
    },

    #[error("Circular dependency: {chain:?}")]
    CircularDependency { chain: Vec<String> },

    #[error("Missing dependency: {name} version {version}")]
    MissingDependency { name: String, version: String },

    #[error("Invalid version requirement: {requirement}")]
    InvalidVersionRequirement { requirement: String },

    #[error("Dependency resolution failed: {reason}")]
    ResolutionFailed { reason: String },
}

/// Convert common error types
impl From<std::io::Error> for ResolveError {
    fn from(err: std::io::Error) -> Self {
        ResolveError::IoError {
            operation: "unknown".to_string(),
            error: err.to_string(),
        }
    }
}

impl From<reqwest::Error> for ResolveError {
    fn from(err: reqwest::Error) -> Self {
        if let Some(url) = err.url() {
            ResolveError::HttpError {
                url: url.to_string(),
                error: err.to_string(),
            }
        } else {
            ResolveError::IoError {
                operation: "http request".to_string(),
                error: err.to_string(),
            }
        }
    }
}

impl From<serde_json::Error> for CacheError {
    fn from(err: serde_json::Error) -> Self {
        CacheError::SerializationError {
            operation: "json".to_string(),
            error: err.to_string(),
        }
    }
}

impl From<handlebars::RenderError> for CompileError {
    fn from(err: handlebars::RenderError) -> Self {
        CompileError::TemplateError {
            template: "unknown".to_string(),
            error: err.to_string(),
        }
    }
}

/// Module execution error (replaces ModuleExecutionError)
pub type ModuleExecutionError = ModuleError;

/// Package manager specific errors
#[derive(Error, Debug)]
pub enum PackageManagerError {
    #[error("Package not found: {name}")]
    PackageNotFound { name: String },

    #[error("Package manager not available: {manager}")]
    ManagerNotAvailable { manager: String },

    #[error("Installation failed for {package}: {error}")]
    InstallationFailed { package: String, error: String },

    #[error("Removal failed for {package}: {error}")]
    RemovalFailed { package: String, error: String },

    #[error("Command execution failed: {error}")]
    CommandFailed { error: String },

    #[error("Operation failed: {error}")]
    OperationFailed { error: String },
}

/// Service manager specific errors
#[derive(Error, Debug)]
pub enum ServiceManagerError {
    #[error("Service not found: {name}")]
    ServiceNotFound { name: String },

    #[error("Service manager not available: {manager}")]
    ManagerNotAvailable { manager: String },

    #[error("Service start failed for {service}: {error}")]
    StartFailed { service: String, error: String },

    #[error("Service stop failed for {service}: {error}")]
    StopFailed { service: String, error: String },

    #[error("Service status check failed for {service}: {error}")]
    StatusCheckFailed { service: String, error: String },

    #[error("Command execution failed: {error}")]
    CommandFailed { error: String },

    #[error("Operation failed: {error}")]
    OperationFailed { error: String },
}

impl From<anyhow::Error> for ValidationError {
    fn from(err: anyhow::Error) -> Self {
        ValidationError::CompatibilityError {
            requirement: err.to_string(),
        }
    }
}

// Add From<std::io::Error> implementations for all error types
impl From<std::io::Error> for ModuleError {
    fn from(err: std::io::Error) -> Self {
        ModuleError::ExecutionFailed {
            message: err.to_string(),
        }
    }
}

impl From<std::io::Error> for PackageManagerError {
    fn from(err: std::io::Error) -> Self {
        PackageManagerError::CommandFailed {
            error: err.to_string(),
        }
    }
}

impl From<std::io::Error> for ServiceManagerError {
    fn from(err: std::io::Error) -> Self {
        ServiceManagerError::CommandFailed {
            error: err.to_string(),
        }
    }
}

impl From<shell_words::ParseError> for ModuleError {
    fn from(err: shell_words::ParseError) -> Self {
        ModuleError::InvalidArgs {
            message: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for ModuleError {
    fn from(err: serde_json::Error) -> Self {
        ModuleError::ExecutionFailed {
            message: format!("JSON serialization error: {err}"),
        }
    }
}

impl From<git2::Error> for ModuleError {
    fn from(err: git2::Error) -> Self {
        ModuleError::ExecutionFailed {
            message: format!("Git error: {err}"),
        }
    }
}

impl From<serde_json::Error> for ValidationError {
    fn from(err: serde_json::Error) -> Self {
        ValidationError::InvalidArgValue {
            arg: "json".to_string(),
            value: "<serialization>".to_string(),
            reason: err.to_string(),
        }
    }
}
