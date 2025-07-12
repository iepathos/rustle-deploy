// Single source of truth for compilation types
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Core compilation types for rustle-deploy.
///
/// This module provides the canonical types for compilation operations,
/// including target specifications, optimization levels, and binary metadata.
/// All other modules should import types from this module to ensure consistency.

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    pub compilation_time: Duration,
    pub optimization_level: OptimizationLevel,
    pub source_info: BinarySourceInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinarySourceInfo {
    pub source_type: BinarySourceType,
    pub template_hash: String,
    pub build_metadata: BuildMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinarySourceType {
    Cache { cache_path: PathBuf },
    FreshCompilation { project_path: PathBuf },
    InMemory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildMetadata {
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub toolchain_version: String,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetSpecification {
    pub target_triple: String,
    pub optimization_level: OptimizationLevel,
    pub platform_info: PlatformInfo,
    pub compilation_options: CompilationOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub architecture: String,
    pub os_family: String,
    pub libc: Option<String>,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationOptions {
    pub strip_debug: bool,
    pub enable_lto: bool,
    pub target_cpu: Option<String>,
    pub custom_features: Vec<String>,
    pub static_linking: bool,
    pub compression: bool,
}

impl Default for CompilationOptions {
    fn default() -> Self {
        Self {
            strip_debug: false,
            enable_lto: false,
            target_cpu: None,
            custom_features: Vec::new(),
            static_linking: true,
            compression: false,
        }
    }
}

// Legacy types for backward compatibility during transition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryCompilation {
    pub compilation_id: String,
    pub binary_name: String,
    pub target_triple: String,
    pub source_tasks: Vec<String>,
    pub embedded_data: EmbeddedExecutionData,
    pub compilation_options: LegacyCompilationOptions,
    pub output_path: PathBuf,
    pub checksum: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedExecutionData {
    pub execution_plan: String,
    pub module_implementations: Vec<ModuleImplementation>,
    pub static_files: Vec<StaticFile>,
    pub runtime_config: crate::runtime::RuntimeConfig,
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
pub struct LegacyCompilationOptions {
    pub optimization_level: OptimizationLevel,
    pub strip_symbols: bool,
    pub static_linking: bool,
    pub compression: bool,
    pub custom_features: Vec<String>,
    pub target_cpu: Option<String>,
}

/// Represents the result of executing a deployed binary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub execution_time: Duration,
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
