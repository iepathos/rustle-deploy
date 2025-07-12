use crate::template::GeneratedTemplate;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::warn;
use uuid::Uuid;

use super::cache::CompilationCache;

#[derive(Error, Debug)]
pub enum CompilationError {
    #[error("Project creation failed: {reason}")]
    ProjectCreationFailed { reason: String },

    #[error("Template writing failed: {file} - {reason}")]
    TemplateWritingFailed { file: String, reason: String },

    #[error("Cargo compilation failed for target {target}: {stderr}")]
    CargoCompilationFailed { target: String, stderr: String },

    #[error("Zigbuild compilation failed for target {target}: {stderr}")]
    ZigbuildCompilationFailed { target: String, stderr: String },

    #[error("Binary not found after compilation: {expected_path}")]
    BinaryNotFound { expected_path: String },

    #[error("Compilation timeout exceeded: {timeout_secs}s")]
    CompilationTimeout { timeout_secs: u64 },

    #[error("Unsupported target architecture: {target}")]
    UnsupportedTarget { target: String },

    #[error("Cache corruption detected: {cache_path}")]
    CacheCorruption { cache_path: String },

    #[error("Insufficient disk space: required {required}MB, available {available}MB")]
    InsufficientDiskSpace { required: u64, available: u64 },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Project error: {0}")]
    Project(#[from] ProjectError),

    #[error("Process execution error: {0}")]
    ProcessExecution(String),

    #[error("General error: {0}")]
    Anyhow(#[from] anyhow::Error),
}

#[derive(Error, Debug)]
pub enum ProjectError {
    #[error("Failed to create project directory: {path} - {reason}")]
    DirectoryCreationFailed { path: String, reason: String },

    #[error("Failed to write file: {file} - {reason}")]
    FileWriteFailed { file: String, reason: String },

    #[error("Template validation failed: {reason}")]
    TemplateValidationFailed { reason: String },

    #[error("Project cleanup failed: {project_id} - {reason}")]
    CleanupFailed { project_id: String, reason: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct BinaryCompiler {
    config: CompilerConfig,
    cache: CompilationCache,
    project_manager: ProjectManager,
    process_executor: ProcessExecutor,
}

#[derive(Debug, Clone)]
pub struct CompilerConfig {
    pub temp_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub compilation_timeout: Duration,
    pub max_parallel_compilations: usize,
    pub enable_cache: bool,
    pub default_optimization: OptimizationLevel,
    pub zigbuild_fallback: bool,
    pub binary_size_limit: Option<u64>,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            temp_dir: std::env::temp_dir().join("rustle-compilation"),
            cache_dir: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".rustle")
                .join("cache"),
            compilation_timeout: Duration::from_secs(300), // 5 minutes
            max_parallel_compilations: num_cpus::get(),
            enable_cache: true,
            default_optimization: OptimizationLevel::Release,
            zigbuild_fallback: true,
            binary_size_limit: Some(50 * 1024 * 1024), // 50MB
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledBinary {
    pub binary_id: String,
    pub target_triple: String,
    pub binary_path: PathBuf,
    pub binary_data: Vec<u8>,
    pub effective_source: BinarySource,
    pub size: u64,
    pub checksum: String,
    pub compilation_time: Duration,
    pub optimization_level: OptimizationLevel,
    pub template_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinarySource {
    FreshCompilation { project_path: PathBuf },
    Cache { cache_path: PathBuf },
    InMemory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetSpecification {
    pub target_triple: String,
    pub optimization_level: OptimizationLevel,
    pub strip_debug: bool,
    pub enable_lto: bool,
    pub target_cpu: Option<String>,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationLevel {
    Debug,
    Release,
    ReleaseWithDebugInfo,
    MinimalSize,
}

#[derive(Debug, Clone)]
pub struct ProjectManager {
    temp_dir: PathBuf,
    template_writer: TemplateWriter,
}

#[derive(Debug, Clone)]
pub struct RustProject {
    pub project_id: String,
    pub project_dir: PathBuf,
    pub cargo_toml_path: PathBuf,
    pub main_rs_path: PathBuf,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ProcessExecutor {
    zigbuild_available: bool,
    cargo_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct TemplateWriter {
    file_writer: FileWriter,
}

#[derive(Debug, Clone)]
pub struct FileWriter;

#[derive(Debug, Clone)]
pub struct CargoTomlGenerator;

impl BinaryCompiler {
    pub fn new(config: CompilerConfig) -> Self {
        let cache = CompilationCache::new(config.cache_dir.clone(), config.enable_cache);
        let project_manager = ProjectManager::new(config.temp_dir.clone());
        let process_executor = ProcessExecutor::new();

        Self {
            config,
            cache,
            project_manager,
            process_executor,
        }
    }

    pub async fn compile_binary(
        &mut self,
        template: &GeneratedTemplate,
        target_spec: &TargetSpecification,
    ) -> Result<CompiledBinary, CompilationError> {
        let compilation_start = Instant::now();

        // Calculate template hash for caching
        let template_hash = self.calculate_template_hash(template)?;

        // Check cache first
        if let Some(cached) = self.check_cache(&template_hash, &target_spec.target_triple) {
            tracing::info!(
                "Found cached binary for template {} target {}",
                template_hash,
                target_spec.target_triple
            );
            return Ok(cached);
        }

        tracing::info!(
            "Compiling binary for target {} with optimization {:?}",
            target_spec.target_triple,
            target_spec.optimization_level
        );

        // Create temporary Rust project
        let project = self.project_manager.create_rust_project(template).await?;

        // Write template to project files
        self.project_manager
            .write_template_to_project(&project, template)
            .await?;

        // Compile the project
        let binary_path = self
            .process_executor
            .compile_project(&project, target_spec, self.config.zigbuild_fallback)
            .await?;

        // Read binary data and create CompiledBinary
        let binary_data = tokio::fs::read(&binary_path).await?;
        let checksum = format!("{:x}", sha2::Sha256::digest(&binary_data));

        let compiled = CompiledBinary {
            binary_id: format!("binary-{}", Uuid::new_v4()),
            target_triple: target_spec.target_triple.clone(),
            binary_path: binary_path.clone(),
            binary_data: binary_data.clone(),
            effective_source: BinarySource::FreshCompilation {
                project_path: binary_path.clone(),
            },
            size: binary_data.len() as u64,
            checksum,
            compilation_time: compilation_start.elapsed(),
            optimization_level: target_spec.optimization_level.clone(),
            template_hash,
            created_at: Utc::now(),
        };

        // Check binary size limit
        if let Some(size_limit) = self.config.binary_size_limit {
            if compiled.size > size_limit {
                tracing::warn!("Binary size {} exceeds limit {}", compiled.size, size_limit);
            }
        }

        // Cache the result
        if self.config.enable_cache {
            if let Err(e) = self.cache.store_binary(&compiled).await {
                warn!("Failed to cache binary: {}", e);
            }
        }

        // Cleanup temporary project
        self.project_manager.cleanup_project(&project).await?;

        tracing::info!(
            "Binary compiled successfully in {:?} (size: {} bytes)",
            compiled.compilation_time,
            compiled.size
        );

        Ok(compiled)
    }

    pub fn check_cache(&self, template_hash: &str, target: &str) -> Option<CompiledBinary> {
        if !self.config.enable_cache {
            return None;
        }

        self.cache.get_binary(template_hash, target)
    }

    pub async fn cleanup_temp_projects(&self) -> Result<(), std::io::Error> {
        self.project_manager.cleanup_all_projects().await
    }

    pub fn cache(&self) -> &CompilationCache {
        &self.cache
    }

    fn calculate_template_hash(&self, template: &GeneratedTemplate) -> Result<String> {
        let mut hasher = sha2::Sha256::new();

        // Hash the template cache key
        hasher.update(&template.cache_key);

        // Hash target info
        hasher.update(&template.target_info.target_triple);
        hasher.update(&template.target_info.architecture);
        hasher.update(&template.target_info.os_family);

        // Hash optimization flags
        for flag in &template.compilation_flags {
            hasher.update(flag);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }
}

impl ProjectManager {
    pub fn new(temp_dir: PathBuf) -> Self {
        let template_writer = TemplateWriter::new();
        Self {
            temp_dir,
            template_writer,
        }
    }

    pub async fn create_rust_project(
        &self,
        _template: &GeneratedTemplate,
    ) -> Result<RustProject, ProjectError> {
        let project_id = format!("rustle-{}", Uuid::new_v4());
        let project_dir = self.temp_dir.join(&project_id);

        // Create project directory structure
        tokio::fs::create_dir_all(&project_dir).await.map_err(|e| {
            ProjectError::DirectoryCreationFailed {
                path: project_dir.display().to_string(),
                reason: e.to_string(),
            }
        })?;

        tokio::fs::create_dir_all(project_dir.join("src"))
            .await
            .map_err(|e| ProjectError::DirectoryCreationFailed {
                path: project_dir.join("src").display().to_string(),
                reason: e.to_string(),
            })?;

        let cargo_toml_path = project_dir.join("Cargo.toml");
        let main_rs_path = project_dir.join("src").join("main.rs");

        tracing::debug!("Created Rust project: {}", project_dir.display());

        Ok(RustProject {
            project_id,
            project_dir,
            cargo_toml_path,
            main_rs_path,
            created_at: Utc::now(),
        })
    }

    pub async fn write_template_to_project(
        &self,
        project: &RustProject,
        template: &GeneratedTemplate,
    ) -> Result<(), ProjectError> {
        self.template_writer
            .write_template_to_project(project, template)
            .await
    }

    pub async fn cleanup_project(&self, project: &RustProject) -> Result<(), ProjectError> {
        tokio::fs::remove_dir_all(&project.project_dir)
            .await
            .map_err(|e| ProjectError::CleanupFailed {
                project_id: project.project_id.clone(),
                reason: e.to_string(),
            })?;

        tracing::debug!("Cleaned up project: {}", project.project_id);
        Ok(())
    }

    pub async fn cleanup_all_projects(&self) -> Result<(), std::io::Error> {
        if self.temp_dir.exists() {
            tokio::fs::remove_dir_all(&self.temp_dir).await?;
            tokio::fs::create_dir_all(&self.temp_dir).await?;
        }
        Ok(())
    }
}

impl Default for ProcessExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessExecutor {
    pub fn new() -> Self {
        let zigbuild_available = which::which("cargo-zigbuild").is_ok();
        let cargo_path = which::which("cargo").unwrap_or_else(|_| PathBuf::from("cargo"));

        if zigbuild_available {
            tracing::info!("cargo-zigbuild detected, will use for cross-compilation");
        } else {
            tracing::warn!("cargo-zigbuild not found, falling back to standard cargo");
        }

        Self {
            zigbuild_available,
            cargo_path,
        }
    }

    pub async fn compile_project(
        &self,
        project: &RustProject,
        target_spec: &TargetSpecification,
        zigbuild_fallback: bool,
    ) -> Result<PathBuf, CompilationError> {
        let binary_path = if self.zigbuild_available {
            // Try zigbuild first
            match self.execute_cargo_zigbuild(
                &project.project_dir,
                &target_spec.target_triple,
                &target_spec.optimization_level,
            )
            .await {
                Ok(path) => path,
                Err(zigbuild_error) => {
                    if zigbuild_fallback {
                        tracing::warn!("Zigbuild failed, falling back to standard cargo: {}", zigbuild_error);
                        self.execute_cargo_build(
                            &project.project_dir,
                            &target_spec.target_triple,
                            &target_spec.optimization_level,
                        )
                        .await?
                    } else {
                        return Err(zigbuild_error);
                    }
                }
            }
        } else {
            self.execute_cargo_build(
                &project.project_dir,
                &target_spec.target_triple,
                &target_spec.optimization_level,
            )
            .await?
        };

        // Verify binary exists and is executable
        if !binary_path.exists() {
            return Err(CompilationError::BinaryNotFound {
                expected_path: binary_path.display().to_string(),
            });
        }

        Ok(binary_path)
    }

    pub async fn execute_cargo_zigbuild(
        &self,
        project_dir: &std::path::Path,
        target: &str,
        optimization: &OptimizationLevel,
    ) -> Result<PathBuf, CompilationError> {
        let mut cmd = tokio::process::Command::new(&self.cargo_path);

        cmd.arg("zigbuild")
            .arg("--target")
            .arg(target)
            .current_dir(project_dir);

        // Set macOS-specific environment variables for zigbuild first
        if cfg!(target_os = "macos") {
            self.configure_macos_zigbuild_env(&mut cmd)?;
        }

        self.add_optimization_flags(&mut cmd, optimization);

        let output =
            cmd.output()
                .await
                .map_err(|e| CompilationError::ZigbuildCompilationFailed {
                    target: target.to_string(),
                    stderr: e.to_string(),
                })?;

        if !output.status.success() {
            return Err(CompilationError::ZigbuildCompilationFailed {
                target: target.to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        self.determine_binary_path(project_dir, target, optimization)
    }

    fn configure_macos_zigbuild_env(&self, cmd: &mut tokio::process::Command) -> Result<(), CompilationError> {
        // Get macOS SDK path
        let sdk_path = std::process::Command::new("xcrun")
            .args(["--show-sdk-path"])
            .output()
            .map_err(|e| CompilationError::ProcessExecution(format!("Failed to get SDK path: {}", e)))?;

        if !sdk_path.status.success() {
            return Err(CompilationError::ProcessExecution("xcrun --show-sdk-path failed".to_string()));
        }

        let sdk_path_string = String::from_utf8_lossy(&sdk_path.stdout);
        let sdk_path_str = sdk_path_string.trim();
        
        // Set essential macOS environment variables for zigbuild
        cmd.env("SDKROOT", sdk_path_str);
        cmd.env("MACOSX_DEPLOYMENT_TARGET", "11.0");
        
        // Set framework search paths
        let frameworks_path = format!("{}/System/Library/Frameworks", sdk_path_str);
        cmd.env("FRAMEWORK_SEARCH_PATHS", &frameworks_path);
        
        // Set library search paths
        let lib_path = format!("{}/usr/lib", sdk_path_str);
        cmd.env("LIBRARY_PATH", &lib_path);
        
        // Set header search paths
        let include_path = format!("{}/usr/include", sdk_path_str);
        cmd.env("CPATH", &include_path);
        
        // Set Zig-specific environment variables
        cmd.env("ZIG_SYSTEM_LINKER_HACK", "1");
        
        // Set additional linker flags for macOS frameworks
        let framework_flags = format!(
            "-L framework={} -F {}",
            frameworks_path,
            frameworks_path
        );
        
        // Get existing RUSTFLAGS and append framework flags
        let existing_rustflags = std::env::var("RUSTFLAGS").unwrap_or_default();
        let combined_rustflags = if existing_rustflags.is_empty() {
            framework_flags
        } else {
            format!("{} {}", existing_rustflags, framework_flags)
        };
        cmd.env("RUSTFLAGS", combined_rustflags);

        tracing::debug!("Configured macOS zigbuild environment: SDKROOT={}, frameworks={}", 
                       sdk_path_str, frameworks_path);

        Ok(())
    }

    pub async fn execute_cargo_build(
        &self,
        project_dir: &std::path::Path,
        target: &str,
        optimization: &OptimizationLevel,
    ) -> Result<PathBuf, CompilationError> {
        let mut cmd = tokio::process::Command::new(&self.cargo_path);

        cmd.arg("build")
            .arg("--target")
            .arg(target)
            .current_dir(project_dir);

        self.add_optimization_flags(&mut cmd, optimization);

        let output = cmd
            .output()
            .await
            .map_err(|e| CompilationError::CargoCompilationFailed {
                target: target.to_string(),
                stderr: e.to_string(),
            })?;

        if !output.status.success() {
            return Err(CompilationError::CargoCompilationFailed {
                target: target.to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        self.determine_binary_path(project_dir, target, optimization)
    }

    fn add_optimization_flags(
        &self,
        cmd: &mut tokio::process::Command,
        optimization: &OptimizationLevel,
    ) {
        match optimization {
            OptimizationLevel::Release => {
                cmd.arg("--release");
            }
            OptimizationLevel::MinimalSize => {
                cmd.arg("--release");
                self.append_rustflags(cmd, "-C opt-level=z -C lto=fat -C strip=symbols");
            }
            OptimizationLevel::Debug => {
                // Default debug build
            }
            OptimizationLevel::ReleaseWithDebugInfo => {
                cmd.arg("--release");
                self.append_rustflags(cmd, "-C debug-assertions=on");
            }
        }
    }

    fn append_rustflags(&self, cmd: &mut tokio::process::Command, new_flags: &str) {
        // Get existing RUSTFLAGS from system environment
        let existing_flags = std::env::var("RUSTFLAGS").unwrap_or_default();
            
        let combined_flags = if existing_flags.is_empty() {
            new_flags.to_string()
        } else {
            format!("{} {}", existing_flags, new_flags)
        };
        cmd.env("RUSTFLAGS", combined_flags);
    }

    fn determine_binary_path(
        &self,
        project_dir: &std::path::Path,
        target: &str,
        optimization: &OptimizationLevel,
    ) -> Result<PathBuf, CompilationError> {
        let profile_dir = match optimization {
            OptimizationLevel::Debug => "debug",
            _ => "release",
        };

        let target_dir = project_dir
            .join("target")
            .join(target)
            .join(profile_dir);

        // First try the expected location
        let mut expected_binary_path = target_dir.join("rustle-runner");
        if target.contains("windows") {
            expected_binary_path.set_extension("exe");
        }

        if expected_binary_path.exists() {
            return Ok(expected_binary_path);
        }

        // If not found, look in the deps directory for the actual binary
        let deps_dir = target_dir.join("deps");
        if let Ok(entries) = std::fs::read_dir(&deps_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    // Look for rustle_runner binary (with or without hash)
                    if file_name.starts_with("rustle_runner") && !file_name.ends_with(".d") && !file_name.contains(".rcgu.") {
                        // Check if it's an executable (not a library or object file)
                        if let Ok(metadata) = std::fs::metadata(&path) {
                            #[cfg(unix)]
                            {
                                use std::os::unix::fs::PermissionsExt;
                                if metadata.permissions().mode() & 0o111 != 0 {
                                    return Ok(path);
                                }
                            }
                            #[cfg(windows)]
                            {
                                if file_name.ends_with(".exe") {
                                    return Ok(path);
                                }
                            }
                            // On other platforms, assume it's executable if it's not a known non-executable extension
                            #[cfg(not(any(unix, windows)))]
                            {
                                if !file_name.ends_with(".rlib") && !file_name.ends_with(".rmeta") && !file_name.ends_with(".o") {
                                    return Ok(path);
                                }
                            }
                        }
                    }
                }
            }
        }

        // If still not found, return the expected path for error reporting
        Ok(expected_binary_path)
    }
}

impl Default for TemplateWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateWriter {
    pub fn new() -> Self {
        Self {
            file_writer: FileWriter,
        }
    }

    pub async fn write_template_to_project(
        &self,
        project: &RustProject,
        template: &GeneratedTemplate,
    ) -> Result<(), ProjectError> {
        // Write Cargo.toml
        self.file_writer
            .write_file(&project.cargo_toml_path, &template.cargo_toml)
            .await?;

        // Write all source files
        for (relative_path, content) in &template.source_files {
            let full_path = project.project_dir.join(relative_path);

            // Create parent directory if it doesn't exist
            if let Some(parent) = full_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            self.file_writer.write_file(&full_path, content).await?;
        }

        tracing::debug!(
            "Wrote {} source files to project {}",
            template.source_files.len(),
            project.project_id
        );

        Ok(())
    }
}

impl FileWriter {
    pub async fn write_file(
        &self,
        path: &std::path::Path,
        content: &str,
    ) -> Result<(), ProjectError> {
        tokio::fs::write(path, content)
            .await
            .map_err(|e| ProjectError::FileWriteFailed {
                file: path.display().to_string(),
                reason: e.to_string(),
            })
    }
}

impl CargoTomlGenerator {
    pub fn generate_cargo_toml(
        &self,
        template: &GeneratedTemplate,
        _target_spec: &TargetSpecification,
    ) -> Result<String, CompilationError> {
        // For now, use the cargo_toml from the template
        // In the future, this could generate dynamic Cargo.toml based on target_spec
        Ok(template.cargo_toml.clone())
    }
}
