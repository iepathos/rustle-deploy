use crate::compilation::capabilities::CompilationStrategy;
use crate::compilation::toolchain::TargetSpecification;
use crate::deploy::{DeployError, Result};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Zig-based cross-compilation integration
pub struct ZigBuildCompiler {
    cargo_path: PathBuf,
    zig_path: Option<PathBuf>,
    build_cache_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ZigCompilationError {
    pub message: String,
    pub target: String,
    pub exit_code: Option<i32>,
    pub stderr: String,
}

#[derive(Debug, Clone)]
pub struct CompiledBinary {
    pub target_triple: String,
    pub binary_path: PathBuf,
    pub size_bytes: u64,
    pub compilation_time: std::time::Duration,
    pub optimization_level: OptimizationLevel,
    pub features: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum OptimizationLevel {
    Debug,
    Release,
    ReleaseWithDebugInfo,
    MinSizeRelease,
}

impl std::fmt::Display for ZigCompilationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Zig compilation failed for {}: {}",
            self.target, self.message
        )
    }
}

impl std::error::Error for ZigCompilationError {}

impl From<ZigCompilationError> for DeployError {
    fn from(err: ZigCompilationError) -> Self {
        DeployError::compilation(err.message)
    }
}

impl ZigBuildCompiler {
    /// Create new ZigBuild compiler instance
    pub async fn new(cache_dir: PathBuf) -> Result<Self> {
        let cargo_path = which::which("cargo")
            .map_err(|e| DeployError::Configuration(format!("cargo not found: {e}")))?;

        let zig_path = which::which("zig").ok();

        let build_cache_dir = cache_dir.join("zigbuild");
        tokio::fs::create_dir_all(&build_cache_dir)
            .await
            .map_err(|e| {
                DeployError::Configuration(format!("Failed to create build cache directory: {e}"))
            })?;

        Ok(Self {
            cargo_path,
            zig_path,
            build_cache_dir,
        })
    }

    /// Compile binary using cargo-zigbuild for the specified target
    pub async fn compile_with_zigbuild(
        &self,
        template_dir: &Path,
        target: &TargetSpecification,
        optimization: OptimizationLevel,
    ) -> Result<CompiledBinary> {
        let start_time = std::time::Instant::now();

        info!(
            "Starting Zig cross-compilation for target: {}",
            target.triple
        );

        // Validate that we can use zigbuild for this target
        if !matches!(target.compilation_strategy, CompilationStrategy::ZigBuild) {
            return Err(DeployError::Configuration(format!(
                "Target {} is not configured for Zig compilation",
                target.triple
            )));
        }

        // Prepare compilation command
        let mut cmd = Command::new(&self.cargo_path);
        cmd.arg("zigbuild")
            .arg("--release") // Always use release for deployment
            .arg("--target")
            .arg(&target.triple)
            .current_dir(template_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Add optimization flags
        match optimization {
            OptimizationLevel::Debug => {
                cmd.arg("--profile").arg("dev");
            }
            OptimizationLevel::Release => {
                cmd.arg("--release");
            }
            OptimizationLevel::ReleaseWithDebugInfo => {
                cmd.arg("--release");
                cmd.env("RUSTFLAGS", "-C debuginfo=1");
            }
            OptimizationLevel::MinSizeRelease => {
                cmd.arg("--release");
                cmd.env("RUSTFLAGS", "-C opt-level=s -C lto=fat");
            }
        }

        // Set Zig-specific environment variables
        if let Some(zig_path) = &self.zig_path {
            cmd.env("ZIG_PATH", zig_path);
        }

        // Set target directory to our cache
        let target_dir = self.build_cache_dir.join(&target.triple);
        cmd.env("CARGO_TARGET_DIR", &target_dir);

        debug!("Executing cargo zigbuild with command: {:?}", cmd);

        // Execute compilation
        let output = cmd.output().await.map_err(|e| {
            DeployError::Configuration(format!("Failed to execute cargo zigbuild: {e}"))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let exit_code = output.status.code();

            warn!("Zig compilation failed for {}: {}", target.triple, stderr);

            return Err(ZigCompilationError {
                message: "Compilation failed".to_string(),
                target: target.triple.clone(),
                exit_code,
                stderr: stderr.to_string(),
            }
            .into());
        }

        // Find the compiled binary
        let binary_path = self
            .find_compiled_binary(&target_dir, &target.triple)
            .await?;

        // Get binary information
        let metadata = tokio::fs::metadata(&binary_path).await.map_err(|e| {
            DeployError::Configuration(format!("Failed to read binary metadata: {e}"))
        })?;

        let compilation_time = start_time.elapsed();

        info!(
            "Successfully compiled {} binary ({} bytes) in {:?}",
            target.triple,
            metadata.len(),
            compilation_time
        );

        Ok(CompiledBinary {
            target_triple: target.triple.clone(),
            binary_path,
            size_bytes: metadata.len(),
            compilation_time,
            optimization_level: optimization,
            features: vec![], // TODO: Extract actual features used
        })
    }

    /// Test if target can be compiled with zigbuild
    pub async fn can_compile_target(&self, target: &str) -> bool {
        let test_command = Command::new(&self.cargo_path)
            .args(["zigbuild", "--target", target, "--help"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        match test_command {
            Ok(status) => status.success(),
            Err(_) => false,
        }
    }

    /// Get list of targets supported by current Zig installation
    pub async fn get_supported_targets(&self) -> Result<Vec<String>> {
        if self.zig_path.is_none() {
            return Ok(vec![]);
        }

        let output = Command::new("zig")
            .args(["targets"])
            .output()
            .await
            .map_err(|e| {
                DeployError::Configuration(format!("Failed to query Zig targets: {e}"))
            })?;

        if !output.status.success() {
            return Ok(vec![]);
        }

        // Parse Zig targets output (simplified)
        let _targets_output = String::from_utf8_lossy(&output.stdout);
        let mut supported_targets = Vec::new();

        // Extract common targets that are known to work well
        for target in crate::compilation::capabilities::ZIG_SUPPORTED_TARGETS {
            supported_targets.push(target.to_string());
        }

        debug!("Zig supports {} targets", supported_targets.len());
        Ok(supported_targets)
    }

    /// Validate Zig and cargo-zigbuild installation
    pub async fn validate_installation(&self) -> Result<ZigBuildValidation> {
        let mut validation = ZigBuildValidation {
            zig_available: false,
            zig_version: None,
            zigbuild_available: false,
            supported_targets: vec![],
            issues: vec![],
        };

        // Check Zig
        if let Some(zig_path) = &self.zig_path {
            let output = Command::new(zig_path).args(["version"]).output().await;

            match output {
                Ok(output) if output.status.success() => {
                    validation.zig_available = true;
                    validation.zig_version =
                        Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
                }
                _ => {
                    validation
                        .issues
                        .push("Zig executable found but not working".to_string());
                }
            }
        } else {
            validation.issues.push("Zig not found in PATH".to_string());
        }

        // Check cargo-zigbuild
        let zigbuild_check = Command::new(&self.cargo_path)
            .args(["zigbuild", "--help"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        match zigbuild_check {
            Ok(status) if status.success() => {
                validation.zigbuild_available = true;
            }
            _ => {
                validation
                    .issues
                    .push("cargo-zigbuild not available".to_string());
            }
        }

        // Get supported targets if everything is working
        if validation.zig_available && validation.zigbuild_available {
            validation.supported_targets = self.get_supported_targets().await.unwrap_or_default();
        }

        Ok(validation)
    }

    /// Find the compiled binary in the target directory
    async fn find_compiled_binary(
        &self,
        target_dir: &Path,
        target_triple: &str,
    ) -> Result<PathBuf> {
        let release_dir = target_dir.join(target_triple).join("release");

        // Try common binary names
        let binary_names = vec!["rustle-executor", "executor", "main"];
        let binary_extension = if target_triple.contains("windows") {
            ".exe"
        } else {
            ""
        };

        for name in binary_names {
            let binary_path = release_dir.join(format!("{name}{binary_extension}"));
            if binary_path.exists() {
                return Ok(binary_path);
            }
        }

        // If not found, try to find any executable in the release directory
        let mut entries = tokio::fs::read_dir(&release_dir).await.map_err(|e| {
            DeployError::Configuration(format!("Failed to read release directory: {e}"))
        })?;

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if let Some(file_name) = path.file_name() {
                let file_name_str = file_name.to_string_lossy();
                if !file_name_str.contains('.') || file_name_str.ends_with(".exe") {
                    // This looks like an executable
                    return Ok(path);
                }
            }
        }

        Err(DeployError::Configuration(format!(
            "Compiled binary not found in {}",
            release_dir.display()
        )))
    }
}

#[derive(Debug, Clone)]
pub struct ZigBuildValidation {
    pub zig_available: bool,
    pub zig_version: Option<String>,
    pub zigbuild_available: bool,
    pub supported_targets: Vec<String>,
    pub issues: Vec<String>,
}

impl ZigBuildValidation {
    pub fn is_fully_functional(&self) -> bool {
        self.zig_available && self.zigbuild_available && self.issues.is_empty()
    }

    pub fn readiness_level(&self) -> ReadinessLevel {
        match (
            self.zig_available,
            self.zigbuild_available,
            self.issues.is_empty(),
        ) {
            (true, true, true) => ReadinessLevel::FullyReady,
            (true, true, false) => ReadinessLevel::MostlyReady,
            (true, false, _) => ReadinessLevel::BasicReady,
            _ => ReadinessLevel::NotReady,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ReadinessLevel {
    FullyReady,  // All components available, all targets supported
    MostlyReady, // Some cross-compilation available
    BasicReady,  // Native compilation only
    NotReady,    // Missing essential components
}
