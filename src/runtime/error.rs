use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("Task execution failed: {task_id}")]
    TaskFailed { task_id: String, reason: String },

    #[error("Module not found: {module}")]
    ModuleNotFound { module: String },

    #[error("Dependency cycle detected: {cycle:?}")]
    DependencyCycle { cycle: Vec<String> },

    #[error("Task timeout: {task_id} ({timeout}s)")]
    TaskTimeout { task_id: String, timeout: u64 },

    #[error("Condition evaluation failed: {condition}")]
    ConditionFailed { condition: String },

    #[error("Facts collection failed: {reason}")]
    FactsCollectionFailed { reason: String },

    #[error("Controller communication failed: {reason}")]
    ControllerCommunicationFailed { reason: String },

    #[error("Invalid execution plan: {reason}")]
    InvalidExecutionPlan { reason: String },

    #[error("Module execution error: {0}")]
    ModuleExecution(#[from] crate::modules::ModuleExecutionError),

    #[error("Timeout error: {0}")]
    Timeout(#[from] tokio::time::error::Elapsed),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Report error: {0}")]
    Report(#[from] ReportError),
}

#[derive(Debug, Error)]
pub enum FactsError {
    #[error("Command failed: {command} - {error}")]
    CommandFailed { command: String, error: String },

    #[error("Parse error: {reason}")]
    ParseError { reason: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("System error: {reason}")]
    SystemError { reason: String },
}

#[derive(Debug, Error)]
pub enum ReportError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Communication failed: {reason}")]
    CommunicationFailed { reason: String },
}

#[derive(Debug, Error)]
pub enum CleanupError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Cleanup failed: {reason}")]
    CleanupFailed { reason: String },
}
