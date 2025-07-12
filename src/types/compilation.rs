// Single source of truth for compilation types
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Core compilation types for rustle-deploy.
///
/// This module provides the canonical types for compilation operations,
/// including target specifications, optimization levels, and binary metadata.
/// All other modules should import types from this module to ensure consistency.
///
/// Canonical optimization level for all compilation operations.
///
/// This enum replaces all other `OptimizationLevel` definitions
/// throughout the codebase. All modules should import this type
/// from `crate::types::compilation`.
///
/// # Examples
///
/// ```rust
/// use crate::types::compilation::OptimizationLevel;
///
/// let opt_level = OptimizationLevel::Release;
/// assert!(matches!(opt_level, OptimizationLevel::Release));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OptimizationLevel {
    /// Debug build with no optimizations
    Debug,
    /// Standard release build with optimizations
    Release,
    /// Release build with debug information
    ReleaseWithDebugInfo,
    /// Optimized for minimal binary size
    MinSize,
    /// Legacy alias for MinSize (from zigbuild module)
    MinSizeRelease,
    /// Legacy alias for MinSize (from compiler module)
    MinimalSize,
    /// Legacy alias for Release (from template generator)
    Aggressive,
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

/// Unified target specification for all compilation operations.
///
/// This struct replaces all other `TargetSpecification` definitions
/// throughout the codebase and includes all functionality from legacy types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetSpecification {
    pub target_triple: String,
    pub optimization_level: OptimizationLevel,
    pub platform_info: PlatformInfo,
    pub compilation_options: CompilationOptions,
    // Additional fields from legacy TargetSpecification types
    pub strip_debug: bool,
    pub enable_lto: bool,
    pub target_cpu: Option<String>,
    pub features: Vec<String>,
    // Fields from toolchain TargetSpecification
    pub platform: Platform,
    pub architecture: Architecture,
    pub requires_zig: bool,
    pub compilation_strategy: CompilationStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub architecture: String,
    pub os_family: String,
    pub libc: Option<String>,
    pub features: Vec<String>,
}

/// Platform enumeration for target specifications
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Platform {
    Linux,
    MacOS,
    Windows,
    Unknown,
}

/// Architecture enumeration for target specifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Architecture {
    X86_64,
    Aarch64,
    X86,
    Arm,
    Unknown,
}

/// Compilation strategy enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompilationStrategy {
    Native,
    CrossCompile,
    ZigBuild,
    Emulation,
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

/// Backend configuration for compilation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerBackendConfig {
    pub backend_type: CompilerBackend,
    pub target_spec: TargetSpecification,
    pub cache_config: CacheConfig,
    pub output_config: OutputConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompilerBackend {
    Cargo,
    ZigBuild { zig_path: PathBuf },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub cache_dir: PathBuf,
    pub enable_cache: bool,
    pub max_cache_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub output_dir: PathBuf,
    pub binary_name: Option<String>,
    pub compression: bool,
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

// Type aliases for gradual migration
pub type LegacyOptimizationLevel = OptimizationLevel;
pub type LegacyTargetSpec = TargetSpecification;

/// Type unification errors
#[derive(Debug, thiserror::Error)]
pub enum TypeUnificationError {
    #[error("Incompatible optimization level: {0}")]
    IncompatibleOptimizationLevel(String),

    #[error("Invalid target specification: {reason}")]
    InvalidTargetSpec { reason: String },

    #[error("Legacy type conversion failed: {source}")]
    ConversionFailed { source: Box<dyn std::error::Error> },
}

// Implementation methods for OptimizationLevel
impl OptimizationLevel {
    /// Check if this is a release variant
    pub fn is_release(&self) -> bool {
        matches!(
            self,
            OptimizationLevel::Release
                | OptimizationLevel::ReleaseWithDebugInfo
                | OptimizationLevel::Aggressive
        )
    }

    /// Check if this is a size-optimized variant
    pub fn is_size_optimized(&self) -> bool {
        matches!(
            self,
            OptimizationLevel::MinSize
                | OptimizationLevel::MinSizeRelease
                | OptimizationLevel::MinimalSize
        )
    }

    /// Convert to the canonical optimization level (resolve aliases)
    pub fn canonical(&self) -> OptimizationLevel {
        match self {
            OptimizationLevel::MinSizeRelease | OptimizationLevel::MinimalSize => {
                OptimizationLevel::MinSize
            }
            OptimizationLevel::Aggressive => OptimizationLevel::Release,
            other => other.clone(),
        }
    }
}

// Implementation methods for TargetSpecification
impl TargetSpecification {
    /// Create a new target specification with defaults
    pub fn new(target_triple: impl Into<String>) -> Self {
        let target_triple = target_triple.into();
        let platform = Platform::from_target_triple(&target_triple);
        let architecture = Architecture::from_target_triple(&target_triple);

        Self {
            target_triple: target_triple.clone(),
            optimization_level: OptimizationLevel::Release,
            platform_info: PlatformInfo {
                architecture: architecture.to_string(),
                os_family: platform.to_string(),
                libc: None,
                features: Vec::new(),
            },
            compilation_options: CompilationOptions::default(),
            strip_debug: false,
            enable_lto: false,
            target_cpu: None,
            features: Vec::new(),
            platform: platform.clone(),
            architecture,
            requires_zig: platform != Platform::from_current_host(),
            compilation_strategy: if platform == Platform::from_current_host() {
                CompilationStrategy::Native
            } else {
                CompilationStrategy::ZigBuild
            },
        }
    }

    /// Validate the target specification
    pub fn validate(&self) -> Result<(), TypeUnificationError> {
        if self.target_triple.is_empty() {
            return Err(TypeUnificationError::InvalidTargetSpec {
                reason: "Empty target triple".to_string(),
            });
        }
        Ok(())
    }

    /// Check if this target requires cross-compilation
    pub fn requires_cross_compilation(&self) -> bool {
        self.platform != Platform::from_current_host() || self.requires_zig
    }
}

// Implementation methods for Platform
impl Platform {
    /// Get the platform for the current host
    pub fn from_current_host() -> Self {
        if cfg!(target_os = "linux") {
            Platform::Linux
        } else if cfg!(target_os = "macos") {
            Platform::MacOS
        } else if cfg!(target_os = "windows") {
            Platform::Windows
        } else {
            Platform::Unknown
        }
    }

    /// Parse platform from target triple
    pub fn from_target_triple(triple: &str) -> Self {
        if triple.contains("linux") {
            Platform::Linux
        } else if triple.contains("darwin") || triple.contains("apple") {
            Platform::MacOS
        } else if triple.contains("windows") || triple.contains("pc-windows") {
            Platform::Windows
        } else {
            Platform::Unknown
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::Linux => write!(f, "linux"),
            Platform::MacOS => write!(f, "macos"),
            Platform::Windows => write!(f, "windows"),
            Platform::Unknown => write!(f, "unknown"),
        }
    }
}

// Implementation methods for Architecture
impl Architecture {
    /// Parse architecture from target triple
    pub fn from_target_triple(triple: &str) -> Self {
        if triple.starts_with("x86_64") {
            Architecture::X86_64
        } else if triple.starts_with("aarch64") {
            Architecture::Aarch64
        } else if triple.starts_with("i686") || triple.starts_with("i586") {
            Architecture::X86
        } else if triple.starts_with("arm") {
            Architecture::Arm
        } else {
            Architecture::Unknown
        }
    }
}

impl std::fmt::Display for Architecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Architecture::X86_64 => write!(f, "x86_64"),
            Architecture::Aarch64 => write!(f, "aarch64"),
            Architecture::X86 => write!(f, "x86"),
            Architecture::Arm => write!(f, "arm"),
            Architecture::Unknown => write!(f, "unknown"),
        }
    }
}
