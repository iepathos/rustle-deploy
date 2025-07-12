/// Cargo-based compilation backend
use super::traits::{BackendCapabilities, CompilationBackend};
use crate::template::GeneratedTemplate;
use crate::types::compilation::{
    BinarySourceInfo, BinarySourceType, BuildMetadata, CompiledBinary, OptimizationLevel,
    TargetSpecification,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Instant;
use tokio::process::Command;
use tracing::{debug, info};

#[derive(Debug, Clone)]
pub struct CargoBackend {
    #[allow(dead_code)]
    cache_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct CargoConfig {
    pub enable_incremental: bool,
    pub verbose: bool,
    pub target_dir: Option<PathBuf>,
}

impl Default for CargoConfig {
    fn default() -> Self {
        Self {
            enable_incremental: true,
            verbose: false,
            target_dir: None,
        }
    }
}

impl Default for CargoBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CargoBackend {
    pub fn new() -> Self {
        Self {
            cache_dir: dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("rustle-deploy")
                .join("cargo"),
        }
    }

    pub fn with_cache_dir(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    fn optimization_level_to_profile(&self, level: &OptimizationLevel) -> &'static str {
        match level {
            OptimizationLevel::Debug => "dev",
            OptimizationLevel::Release => "release",
            OptimizationLevel::ReleaseWithDebugInfo => "release",
            OptimizationLevel::MinSize => "release", // Will use additional flags
        }
    }

    async fn run_cargo_build(
        &self,
        project_path: &std::path::Path,
        target: &TargetSpecification,
        config: &CargoConfig,
    ) -> Result<PathBuf> {
        let mut cmd = Command::new("cargo");
        cmd.arg("build");

        // Set optimization profile
        let profile = self.optimization_level_to_profile(&target.optimization_level);
        if profile == "release" {
            cmd.arg("--release");
        }

        // Set target triple
        cmd.args(["--target", &target.target_triple]);

        // Target directory
        if let Some(target_dir) = &config.target_dir {
            cmd.args(["--target-dir", &target_dir.to_string_lossy()]);
        }

        // Verbose output
        if config.verbose {
            cmd.arg("--verbose");
        }

        // Additional options for MinSize
        if target.optimization_level == OptimizationLevel::MinSize {
            cmd.env("RUSTFLAGS", "-C opt-level=z -C lto=fat -C codegen-units=1");
        }

        // LTO settings
        if target.compilation_options.enable_lto {
            let mut rustflags = std::env::var("RUSTFLAGS").unwrap_or_default();
            if !rustflags.is_empty() {
                rustflags.push(' ');
            }
            rustflags.push_str("-C lto=fat");
            cmd.env("RUSTFLAGS", rustflags);
        }

        // Set working directory
        cmd.current_dir(project_path);

        debug!("Running cargo command: {:?}", cmd);

        let output = cmd
            .output()
            .await
            .context("Failed to execute cargo build")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Cargo build failed: {}", stderr);
        }

        // Find the built binary
        let target_dir = config
            .target_dir
            .clone()
            .unwrap_or_else(|| project_path.join("target"));

        let binary_path = target_dir
            .join(&target.target_triple)
            .join(profile)
            .join("rustle-binary"); // Assuming binary name

        if !binary_path.exists() {
            anyhow::bail!("Built binary not found at: {}", binary_path.display());
        }

        Ok(binary_path)
    }

    async fn get_toolchain_version(&self) -> Result<String> {
        let output = Command::new("rustc")
            .arg("--version")
            .output()
            .await
            .context("Failed to get rustc version")?;

        if !output.status.success() {
            anyhow::bail!("Failed to get rustc version");
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

#[async_trait]
impl CompilationBackend for CargoBackend {
    type Error = anyhow::Error;
    type Config = serde_json::Value;

    async fn compile_binary(
        &self,
        template: &GeneratedTemplate,
        target: &TargetSpecification,
        _config: &Self::Config,
    ) -> Result<CompiledBinary> {
        let start_time = Instant::now();

        info!(
            "Starting Cargo compilation for target: {}",
            target.target_triple
        );

        // Create temporary project directory
        let temp_dir = tempfile::tempdir().context("Failed to create temporary directory")?;
        let project_path = temp_dir.path();

        // Write template files to project directory
        template
            .write_to_directory(project_path)
            .await
            .context("Failed to write template to directory")?;

        // Use default config for now
        let config = CargoConfig::default();
        // Run cargo build
        let binary_path = self.run_cargo_build(project_path, target, &config).await?;

        // Read binary data
        let binary_data = tokio::fs::read(&binary_path)
            .await
            .context("Failed to read compiled binary")?;

        let size = binary_data.len() as u64;

        // Calculate checksum
        let checksum = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(&binary_data);
            format!("{:x}", hasher.finalize())
        };

        let compilation_time = start_time.elapsed();

        // Get toolchain version
        let toolchain_version = self
            .get_toolchain_version()
            .await
            .unwrap_or_else(|_| "unknown".to_string());

        // Create source info
        let source_info = BinarySourceInfo {
            source_type: BinarySourceType::FreshCompilation {
                project_path: project_path.to_path_buf(),
            },
            template_hash: template.calculate_hash(),
            build_metadata: BuildMetadata {
                created_at: chrono::Utc::now(),
                toolchain_version,
                features: target.compilation_options.custom_features.clone(),
            },
        };

        let compiled_binary = CompiledBinary {
            compilation_id: uuid::Uuid::new_v4().to_string(),
            target_triple: target.target_triple.clone(),
            binary_data,
            checksum,
            size,
            compilation_time,
            optimization_level: target.optimization_level.clone(),
            source_info,
        };

        info!(
            "Cargo compilation completed in {:?}, binary size: {} bytes",
            compilation_time, size
        );

        Ok(compiled_binary)
    }

    fn supports_target(&self, _target: &str) -> bool {
        // Cargo supports most Rust targets
        // In practice, we'd check against `rustup target list` output
        true
    }

    fn get_capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            supported_targets: vec![], // Would be populated from rustc --print target-list
            supports_cross_compilation: true,
            supports_static_linking: true,
            supports_lto: true,
            requires_toolchain: true,
        }
    }

    fn backend_name(&self) -> &'static str {
        "cargo"
    }
}
