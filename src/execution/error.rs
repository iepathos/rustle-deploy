use thiserror::Error;

impl From<ValidationError> for ParseError {
    fn from(err: ValidationError) -> Self {
        ParseError::SchemaValidation {
            errors: vec![err.to_string()],
        }
    }
}

impl From<DependencyError> for ValidationError {
    fn from(err: DependencyError) -> Self {
        ValidationError::InvalidInventory {
            reason: err.to_string(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Invalid JSON format: {reason}")]
    InvalidJson { reason: String },

    #[error("Invalid YAML format: {reason}")]
    InvalidYaml { reason: String },

    #[error("Schema validation failed: {errors:?}")]
    SchemaValidation { errors: Vec<String> },

    #[error("Unknown plan format")]
    UnknownFormat,

    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Invalid field value: {field} = {value}")]
    InvalidFieldValue { field: String, value: String },
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Circular dependency detected: {cycle:?}")]
    CircularDependency { cycle: Vec<String> },

    #[error("Missing task dependency: {task} -> {dependency}")]
    MissingDependency { task: String, dependency: String },

    #[error("Invalid target selector: {selector}")]
    InvalidTargetSelector { selector: String },

    #[error("Unknown module: {module}")]
    UnknownModule { module: String },

    #[error("Invalid inventory format: {reason}")]
    InvalidInventory { reason: String },
}

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("Template processing failed: {reason}")]
    ProcessingFailed { reason: String },

    #[error("Missing template variable: {variable}")]
    MissingVariable { variable: String },

    #[error("Invalid template syntax: {syntax}")]
    InvalidSyntax { syntax: String },
}

#[derive(Debug, Error)]
pub enum ExtractionError {
    #[error("Failed to extract deployment targets: {reason}")]
    ExtractionFailed { reason: String },

    #[error("Invalid inventory data: {reason}")]
    InvalidInventoryData { reason: String },

    #[error("Missing architecture information for host: {host}")]
    MissingArchitecture { host: String },
}

#[derive(Debug, Error)]
pub enum DependencyError {
    #[error("Dependency resolution failed: {reason}")]
    ResolutionFailed { reason: String },

    #[error("Circular dependency cycle: {cycle:?}")]
    CircularDependencies { cycle: Vec<String> },

    #[error("Missing dependency: {missing}")]
    MissingDependency { missing: String },
}

#[derive(Debug, Error)]
pub enum OrderingError {
    #[error("Failed to compute execution order: {reason}")]
    OrderingFailed { reason: String },

    #[error("Topological sort failed: {reason}")]
    TopologicalSortFailed { reason: String },
}
