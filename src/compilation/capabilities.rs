use crate::deploy::{DeployError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Compilation capabilities detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationCapabilities {
    pub rust_version: Option<String>,
    pub zig_available: bool,
    pub zigbuild_available: bool,
    pub available_targets: HashSet<String>,
    pub native_target: String,
    pub capability_level: CapabilityLevel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CapabilityLevel {
    Full,        // Zig + cargo-zigbuild available, all targets supported
    Limited,     // Rust only, native target and some cross-compilation
    Minimal,     // Rust only, native target only
    Insufficient, // Missing requirements
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustInstallation {
    pub version: String,
    pub toolchain: String,
    pub targets: Vec<String>,
    pub cargo_path: PathBuf,
    pub rustc_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZigInstallation {
    pub version: String,
    pub zig_path: PathBuf,
    pub supports_cross_compilation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupRecommendation {
    pub improvement: String,
    pub impact: ImpactLevel,
    pub installation_command: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactLevel {
    Critical, // Required for basic functionality
    High,     // Enables cross-compilation
    Medium,   // Performance improvement
    Low,      // Nice to have
}

impl Default for CompilationCapabilities {
    fn default() -> Self {
        Self {
            rust_version: None,
            zig_available: false,
            zigbuild_available: false,
            available_targets: HashSet::new(),
            native_target: get_native_target().to_string(),
            capability_level: CapabilityLevel::Insufficient,
        }
    }
}

impl CompilationCapabilities {
    /// Quick capability detection without expensive target testing
    pub async fn detect_basic() -> Result<Self> {
        let mut capabilities = Self::default();

        // Check Rust installation
        if let Ok(rust) = detect_rust_installation().await {
            capabilities.rust_version = Some(rust.version);
            capabilities.available_targets.insert(capabilities.native_target.clone());
            capabilities.capability_level = CapabilityLevel::Minimal;
        }

        // Check Zig installation
        if let Ok(Some(_)) = detect_zig_installation().await {
            capabilities.zig_available = true;

            // Check cargo-zigbuild
            if is_zigbuild_available().await.unwrap_or(false) {
                capabilities.zigbuild_available = true;
                capabilities.capability_level = CapabilityLevel::Full;

                // Add common Zig-supported targets
                for target in ZIG_SUPPORTED_TARGETS {
                    capabilities.available_targets.insert(target.to_string());
                }
            }
        }

        debug!("Basic capability detection completed: {:?}", capabilities.capability_level);
        Ok(capabilities)
    }

    /// Full capability detection with target testing
    pub async fn detect_full() -> Result<Self> {
        let mut capabilities = Self::detect_basic().await?;

        // Test common cross-compilation targets if we have basic Rust
        if capabilities.rust_version.is_some() {
            for target in COMMON_TARGETS {
                if test_target_compilation(target).await.unwrap_or(false) {
                    capabilities.available_targets.insert(target.to_string());
                }
            }

            // Update capability level based on available targets
            if capabilities.available_targets.len() > 1 && !capabilities.zigbuild_available {
                capabilities.capability_level = CapabilityLevel::Limited;
            }
        }

        info!("Full capability detection completed: {} targets available", capabilities.available_targets.len());
        Ok(capabilities)
    }

    /// Generate setup recommendations based on current capabilities
    pub fn get_recommendations(&self) -> Vec<SetupRecommendation> {
        let mut recommendations = Vec::new();

        if self.rust_version.is_none() {
            recommendations.push(SetupRecommendation {
                improvement: "Install Rust toolchain".to_string(),
                impact: ImpactLevel::Critical,
                installation_command: Some("curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh".to_string()),
                description: "Rust is required for all compilation functionality".to_string(),
            });
        }

        if !self.zig_available {
            recommendations.push(SetupRecommendation {
                improvement: "Install Zig for enhanced cross-compilation".to_string(),
                impact: ImpactLevel::High,
                installation_command: Some("Visit https://ziglang.org/download/ for installation".to_string()),
                description: "Zig enables zero-infrastructure cross-compilation to all supported targets".to_string(),
            });
        } else if !self.zigbuild_available {
            recommendations.push(SetupRecommendation {
                improvement: "Install cargo-zigbuild".to_string(),
                impact: ImpactLevel::High,
                installation_command: Some("cargo install cargo-zigbuild".to_string()),
                description: "cargo-zigbuild integrates Zig with Cargo for seamless cross-compilation".to_string(),
            });
        }

        if self.available_targets.len() <= 1 {
            recommendations.push(SetupRecommendation {
                improvement: "Add cross-compilation targets".to_string(),
                impact: ImpactLevel::Medium,
                installation_command: Some("rustup target add <target-triple>".to_string()),
                description: "Additional targets enable deployment to diverse architectures".to_string(),
            });
        }

        recommendations
    }

    /// Check if a specific target is supported
    pub fn supports_target(&self, target: &str) -> bool {
        self.available_targets.contains(target)
    }

    /// Get compilation strategy for a target
    pub fn get_strategy_for_target(&self, target: &str) -> CompilationStrategy {
        if !self.supports_target(target) {
            return CompilationStrategy::SshFallback;
        }

        if self.zigbuild_available && ZIG_SUPPORTED_TARGETS.contains(&target) {
            CompilationStrategy::ZigBuild
        } else if target == &self.native_target {
            CompilationStrategy::NativeCargo
        } else {
            CompilationStrategy::SshFallback
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompilationStrategy {
    ZigBuild,
    NativeCargo,
    SshFallback,
}

/// Detect Rust installation details
pub async fn detect_rust_installation() -> Result<RustInstallation> {
    let output = Command::new("rustc")
        .args(["--version"])
        .output()
        .await
        .map_err(|e| DeployError::Configuration(format!("Failed to run rustc: {}", e)))?;

    if !output.status.success() {
        return Err(DeployError::Configuration("rustc command failed".to_string()));
    }

    let version_output = String::from_utf8_lossy(&output.stdout);
    let version = version_output
        .split_whitespace()
        .nth(1)
        .unwrap_or("unknown")
        .to_string();

    // Get toolchain info
    let toolchain_output = Command::new("rustup")
        .args(["show", "active-toolchain"])
        .output()
        .await
        .map_err(|e| DeployError::Configuration(format!("Failed to run rustup: {}", e)))?;

    let toolchain = if toolchain_output.status.success() {
        String::from_utf8_lossy(&toolchain_output.stdout)
            .trim()
            .to_string()
    } else {
        "default".to_string()
    };

    // Get installed targets
    let targets_output = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .await
        .map_err(|e| DeployError::Configuration(format!("Failed to list targets: {}", e)))?;

    let targets = if targets_output.status.success() {
        String::from_utf8_lossy(&targets_output.stdout)
            .lines()
            .map(|line| line.trim().to_string())
            .collect()
    } else {
        vec![get_native_target().to_string()]
    };

    // Locate binaries
    let cargo_path = which::which("cargo")
        .map_err(|e| DeployError::Configuration(format!("cargo not found: {}", e)))?;
    let rustc_path = which::which("rustc")
        .map_err(|e| DeployError::Configuration(format!("rustc not found: {}", e)))?;

    Ok(RustInstallation {
        version,
        toolchain,
        targets,
        cargo_path,
        rustc_path,
    })
}

/// Detect Zig installation
pub async fn detect_zig_installation() -> Result<Option<ZigInstallation>> {
    let output = Command::new("zig")
        .args(["version"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    match output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let zig_path = which::which("zig")
                .map_err(|e| DeployError::Configuration(format!("zig not found in PATH: {}", e)))?;

            Ok(Some(ZigInstallation {
                version,
                zig_path,
                supports_cross_compilation: true, // Zig always supports cross-compilation
            }))
        }
        _ => Ok(None),
    }
}

/// Check if cargo-zigbuild is available
pub async fn is_zigbuild_available() -> Result<bool> {
    let output = Command::new("cargo")
        .args(["zigbuild", "--help"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    match output {
        Ok(output) => Ok(output.status.success()),
        Err(_) => Ok(false),
    }
}

/// Test if a target can be compiled for
async fn test_target_compilation(target: &str) -> Result<bool> {
    let output = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .await
        .map_err(|e| DeployError::Configuration(format!("Failed to list targets: {}", e)))?;

    if !output.status.success() {
        return Ok(false);
    }

    let installed_targets = String::from_utf8_lossy(&output.stdout);
    Ok(installed_targets.lines().any(|line| line.trim() == target))
}

/// Common cross-compilation targets that work with standard Rust
const COMMON_TARGETS: &[&str] = &[
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-gnu",
];

/// Get the native target triple
fn get_native_target() -> &'static str {
    match (std::env::consts::ARCH, std::env::consts::OS) {
        ("x86_64", "linux") => "x86_64-unknown-linux-gnu",
        ("aarch64", "linux") => "aarch64-unknown-linux-gnu",
        ("x86_64", "macos") => "x86_64-apple-darwin",
        ("aarch64", "macos") => "aarch64-apple-darwin",
        ("x86_64", "windows") => "x86_64-pc-windows-msvc",
        _ => "unknown-unknown-unknown",
    }
}

/// Targets that are well-supported by Zig cross-compilation
pub const ZIG_SUPPORTED_TARGETS: &[&str] = &[
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-unknown-linux-musl",
    "aarch64-unknown-linux-musl",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-gnu",
    "i686-unknown-linux-gnu",
    "arm-unknown-linux-gnueabihf",
    "armv7-unknown-linux-gnueabihf",
    "mips64-unknown-linux-gnuabi64",
    "powerpc64le-unknown-linux-gnu",
    "riscv64gc-unknown-linux-gnu",
    "s390x-unknown-linux-gnu",
    "wasm32-wasi",
];