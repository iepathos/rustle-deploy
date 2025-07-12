use crate::compilation::capabilities::{CompilationCapabilities, SetupRecommendation};
use crate::deploy::{DeployError, Result};
use crate::types::compilation::TargetSpecification;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Detects and manages cross-compilation toolchain
pub struct ToolchainDetector {
    cache: DetectionCache,
}

#[derive(Debug, Clone)]
pub struct DetectionCache {
    cached_capabilities: Option<CompilationCapabilities>,
    cache_timestamp: Option<std::time::SystemTime>,
    cache_duration: std::time::Duration,
}

#[derive(Debug, Clone)]
pub enum DetectionError {
    ToolchainMissing(String),
    VersionIncompatible(String),
    InstallationFailed(String),
    PermissionDenied(String),
}

#[derive(Debug, Clone)]
pub enum ToolchainError {
    NotFound(String),
    InvalidVersion(String),
    PermissionDenied,
}

#[derive(Debug, Clone)]
pub enum InstallationError {
    NetworkError(String),
    PermissionDenied,
    DiskSpace,
    UnsupportedPlatform,
}

impl std::fmt::Display for DetectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DetectionError::ToolchainMissing(tool) => write!(f, "Toolchain missing: {tool}"),
            DetectionError::VersionIncompatible(msg) => write!(f, "Version incompatible: {msg}"),
            DetectionError::InstallationFailed(msg) => write!(f, "Installation failed: {msg}"),
            DetectionError::PermissionDenied(msg) => write!(f, "Permission denied: {msg}"),
        }
    }
}

impl std::error::Error for DetectionError {}

impl From<DetectionError> for DeployError {
    fn from(err: DetectionError) -> Self {
        DeployError::Configuration(err.to_string())
    }
}

impl Default for DetectionCache {
    fn default() -> Self {
        Self {
            cached_capabilities: None,
            cache_timestamp: None,
            cache_duration: std::time::Duration::from_secs(3600), // 1 hour cache
        }
    }
}

impl ToolchainDetector {
    pub fn new() -> Self {
        Self {
            cache: DetectionCache::default(),
        }
    }

    /// Detect full capabilities with caching
    pub async fn detect_full_capabilities(&mut self) -> Result<CompilationCapabilities> {
        // Check cache validity
        if let (Some(cached), Some(timestamp)) =
            (&self.cache.cached_capabilities, &self.cache.cache_timestamp)
        {
            if timestamp.elapsed().unwrap_or(std::time::Duration::MAX) < self.cache.cache_duration {
                debug!("Using cached capability detection");
                return Ok(cached.clone());
            }
        }

        info!("Performing full toolchain capability detection");
        let capabilities = CompilationCapabilities::detect_full().await?;

        // Update cache
        self.cache.cached_capabilities = Some(capabilities.clone());
        self.cache.cache_timestamp = Some(std::time::SystemTime::now());

        Ok(capabilities)
    }

    /// Check Rust installation details
    pub async fn check_rust_installation(
        &self,
    ) -> Result<crate::compilation::capabilities::RustInstallation> {
        crate::compilation::capabilities::detect_rust_installation().await
    }

    /// Check Zig installation details
    pub async fn check_zig_installation(
        &self,
    ) -> Result<Option<crate::compilation::capabilities::ZigInstallation>> {
        crate::compilation::capabilities::detect_zig_installation().await
    }

    /// Check if cargo-zigbuild is installed
    pub async fn check_zigbuild_installation(&self) -> Result<bool> {
        crate::compilation::capabilities::is_zigbuild_available().await
    }

    /// Install cargo-zigbuild if missing
    pub async fn install_zigbuild_if_missing(&self) -> Result<()> {
        if self.check_zigbuild_installation().await? {
            debug!("cargo-zigbuild already installed");
            return Ok(());
        }

        info!("Installing cargo-zigbuild");

        let output = Command::new("cargo")
            .args(["install", "cargo-zigbuild"])
            .output()
            .await
            .map_err(|e| DeployError::Configuration(format!("Failed to run cargo install: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DeployError::Configuration(format!(
                "Failed to install cargo-zigbuild: {stderr}"
            )));
        }

        info!("Successfully installed cargo-zigbuild");
        Ok(())
    }

    /// Get supported targets based on current capabilities
    pub fn get_supported_targets(
        &self,
        capabilities: &CompilationCapabilities,
    ) -> Vec<TargetSpecification> {
        let mut targets = Vec::new();

        for target_triple in &capabilities.available_targets {
            let strategy = capabilities.get_strategy_for_target(target_triple);
            let requires_zig = matches!(
                strategy,
                crate::compilation::capabilities::CompilationStrategy::ZigBuild
            );

            let mut target_spec = TargetSpecification::new(target_triple.clone());
            target_spec.requires_zig = requires_zig;
            target_spec.compilation_strategy = strategy.into();
            targets.push(target_spec);
        }

        targets
    }

    /// Generate recommendations for improving the setup
    pub fn recommend_setup_improvements(
        &self,
        capabilities: &CompilationCapabilities,
    ) -> Vec<SetupRecommendation> {
        capabilities.get_recommendations()
    }

    /// Validate that the toolchain is properly configured
    pub async fn validate_toolchain(&self) -> Result<ToolchainStatus> {
        let mut status = ToolchainStatus {
            rust_available: false,
            zig_available: false,
            zigbuild_available: false,
            issues: Vec::new(),
            overall_health: HealthStatus::Poor,
        };

        // Check Rust
        match self.check_rust_installation().await {
            Ok(_) => {
                status.rust_available = true;
                debug!("Rust toolchain validated successfully");
            }
            Err(e) => {
                status.issues.push(format!("Rust toolchain issue: {e}"));
                warn!("Rust toolchain validation failed: {}", e);
            }
        }

        // Check Zig
        match self.check_zig_installation().await {
            Ok(Some(_)) => {
                status.zig_available = true;
                debug!("Zig installation validated successfully");
            }
            Ok(None) => {
                debug!("Zig not installed");
            }
            Err(e) => {
                status.issues.push(format!("Zig detection issue: {e}"));
                warn!("Zig detection failed: {}", e);
            }
        }

        // Check cargo-zigbuild
        match self.check_zigbuild_installation().await {
            Ok(true) => {
                status.zigbuild_available = true;
                debug!("cargo-zigbuild validated successfully");
            }
            Ok(false) => {
                debug!("cargo-zigbuild not installed");
            }
            Err(e) => {
                status
                    .issues
                    .push(format!("cargo-zigbuild detection issue: {e}"));
                warn!("cargo-zigbuild detection failed: {}", e);
            }
        }

        // Determine overall health
        status.overall_health = match (
            status.rust_available,
            status.zig_available,
            status.zigbuild_available,
        ) {
            (true, true, true) => HealthStatus::Excellent,
            (true, _, true) => HealthStatus::Good,
            (true, _, _) => HealthStatus::Fair,
            _ => HealthStatus::Poor,
        };

        Ok(status)
    }
}

// TargetSpecification, Platform, and Architecture moved to crate::types::compilation
// Use: use crate::types::compilation::{TargetSpecification, Platform, Architecture};

#[derive(Debug, Clone)]
pub struct ToolchainStatus {
    pub rust_available: bool,
    pub zig_available: bool,
    pub zigbuild_available: bool,
    pub issues: Vec<String>,
    pub overall_health: HealthStatus,
}

#[derive(Debug, Clone)]
pub enum HealthStatus {
    Excellent, // All tools available and working
    Good,      // Core tools + some cross-compilation
    Fair,      // Basic Rust only
    Poor,      // Missing essential components
}

// Platform and Architecture implementations moved to crate::types::compilation

// Add conversion between CompilationStrategy types
use crate::types::compilation::CompilationStrategy as UnifiedCompilationStrategy;

impl From<crate::compilation::capabilities::CompilationStrategy> for UnifiedCompilationStrategy {
    fn from(strategy: crate::compilation::capabilities::CompilationStrategy) -> Self {
        match strategy {
            crate::compilation::capabilities::CompilationStrategy::NativeCargo => {
                UnifiedCompilationStrategy::Native
            }
            crate::compilation::capabilities::CompilationStrategy::ZigBuild => {
                UnifiedCompilationStrategy::ZigBuild
            }
            crate::compilation::capabilities::CompilationStrategy::SshFallback => {
                UnifiedCompilationStrategy::Emulation
            }
        }
    }
}

impl Default for ToolchainDetector {
    fn default() -> Self {
        Self::new()
    }
}
