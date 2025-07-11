use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryCompilation {
    pub compilation_id: String,
    pub binary_name: String,
    pub target_triple: String,
    pub source_tasks: Vec<String>,
    pub embedded_data: EmbeddedExecutionData,
    pub compilation_options: CompilationOptions,
    pub output_path: PathBuf,
    pub checksum: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedExecutionData {
    pub execution_plan: String,
    pub module_implementations: Vec<ModuleImplementation>,
    pub static_files: Vec<StaticFile>,
    pub runtime_config: super::RuntimeConfig,
    pub facts_template: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleImplementation {
    pub module_name: String,
    pub source_code: String,
    pub dependencies: Vec<String>,
    pub static_linked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticFile {
    pub embedded_path: String,
    pub target_path: String,
    pub content: Vec<u8>,
    pub permissions: u32,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationOptions {
    pub optimization_level: OptimizationLevel,
    pub strip_symbols: bool,
    pub static_linking: bool,
    pub compression: bool,
    pub custom_features: Vec<String>,
    pub target_cpu: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, Hash, PartialEq)]
pub enum OptimizationLevel {
    Debug,
    Release,
    ReleaseWithDebugInfo,
    MinSize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledBinary {
    pub compilation_id: String,
    pub target_triple: String,
    pub binary_data: Vec<u8>,
    pub checksum: String,
    pub size: u64,
    pub compilation_time: std::time::Duration,
}

/// Represents the result of executing a deployed binary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub execution_time: std::time::Duration,
}

/// Configuration for the deployment process
#[derive(Debug, Clone)]
pub struct DeploymentConfig {
    pub cache_dir: PathBuf,
    pub output_dir: PathBuf,
    pub parallel_jobs: usize,
    pub default_timeout_secs: u64,
    pub verify_deployments: bool,
    pub compression: bool,
    pub strip_symbols: bool,
    pub binary_size_limit_mb: u64,
}
