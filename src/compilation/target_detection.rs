use crate::compilation::{OptimizationLevel, TargetSpecification};
use crate::compilation::toolchain::{Architecture, Platform};
use anyhow::Result;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TargetDetectionError {
    #[error("Unsupported target platform: {platform}")]
    UnsupportedPlatform { platform: String },

    #[error("Failed to detect host architecture")]
    ArchitectureDetectionFailed,

    #[error("Invalid target triple: {triple}")]
    InvalidTargetTriple { triple: String },
}

/// Target platform detection and configuration
pub struct TargetDetector {
    supported_targets: HashMap<String, TargetInfo>,
}

#[derive(Debug, Clone)]
pub struct TargetInfo {
    pub target_triple: String,
    pub platform: Platform,
    pub architecture: String,
    pub os_family: String,
    pub libc: Option<String>,
    pub default_features: Vec<String>,
    pub zigbuild_supported: bool,
}

impl TargetDetector {
    pub fn new() -> Self {
        let mut supported_targets = HashMap::new();

        // macOS targets
        supported_targets.insert(
            "aarch64-apple-darwin".to_string(),
            TargetInfo {
                target_triple: "aarch64-apple-darwin".to_string(),
                platform: Platform::MacOS,
                architecture: "aarch64".to_string(),
                os_family: "unix".to_string(),
                libc: None,
                default_features: vec![],
                zigbuild_supported: true,
            },
        );

        supported_targets.insert(
            "x86_64-apple-darwin".to_string(),
            TargetInfo {
                target_triple: "x86_64-apple-darwin".to_string(),
                platform: Platform::MacOS,
                architecture: "x86_64".to_string(),
                os_family: "unix".to_string(),
                libc: None,
                default_features: vec![],
                zigbuild_supported: true,
            },
        );

        // Linux targets
        supported_targets.insert(
            "x86_64-unknown-linux-gnu".to_string(),
            TargetInfo {
                target_triple: "x86_64-unknown-linux-gnu".to_string(),
                platform: Platform::Linux,
                architecture: "x86_64".to_string(),
                os_family: "unix".to_string(),
                libc: Some("gnu".to_string()),
                default_features: vec![],
                zigbuild_supported: true,
            },
        );

        supported_targets.insert(
            "aarch64-unknown-linux-gnu".to_string(),
            TargetInfo {
                target_triple: "aarch64-unknown-linux-gnu".to_string(),
                platform: Platform::Linux,
                architecture: "aarch64".to_string(),
                os_family: "unix".to_string(),
                libc: Some("gnu".to_string()),
                default_features: vec![],
                zigbuild_supported: true,
            },
        );

        supported_targets.insert(
            "x86_64-unknown-linux-musl".to_string(),
            TargetInfo {
                target_triple: "x86_64-unknown-linux-musl".to_string(),
                platform: Platform::Linux,
                architecture: "x86_64".to_string(),
                os_family: "unix".to_string(),
                libc: Some("musl".to_string()),
                default_features: vec![],
                zigbuild_supported: true,
            },
        );

        // Windows targets
        supported_targets.insert(
            "x86_64-pc-windows-msvc".to_string(),
            TargetInfo {
                target_triple: "x86_64-pc-windows-msvc".to_string(),
                platform: Platform::Windows,
                architecture: "x86_64".to_string(),
                os_family: "windows".to_string(),
                libc: None,
                default_features: vec![],
                zigbuild_supported: false, // Windows cross-compilation is complex
            },
        );

        Self { supported_targets }
    }

    /// Detect the current host target triple
    pub fn detect_host_target(&self) -> Result<String, TargetDetectionError> {
        let target_triple = if cfg!(target_arch = "aarch64") && cfg!(target_os = "macos") {
            "aarch64-apple-darwin"
        } else if cfg!(target_arch = "x86_64") && cfg!(target_os = "macos") {
            "x86_64-apple-darwin"
        } else if cfg!(target_arch = "x86_64") && cfg!(target_os = "linux") {
            "x86_64-unknown-linux-gnu"
        } else if cfg!(target_arch = "aarch64") && cfg!(target_os = "linux") {
            "aarch64-unknown-linux-gnu"
        } else if cfg!(target_arch = "x86_64") && cfg!(target_os = "windows") {
            "x86_64-pc-windows-msvc"
        } else {
            return Err(TargetDetectionError::ArchitectureDetectionFailed);
        };

        Ok(target_triple.to_string())
    }

    /// Create a target specification for localhost testing
    pub fn create_localhost_target_spec(
        &self,
    ) -> Result<TargetSpecification, TargetDetectionError> {
        let host_target = self.detect_host_target()?;
        self.create_target_spec(&host_target, OptimizationLevel::Release)
    }

    /// Create a target specification for a given target triple
    pub fn create_target_spec(
        &self,
        target_triple: &str,
        optimization_level: OptimizationLevel,
    ) -> Result<TargetSpecification, TargetDetectionError> {
        let target_info = self.supported_targets.get(target_triple).ok_or_else(|| {
            TargetDetectionError::UnsupportedPlatform {
                platform: target_triple.to_string(),
            }
        })?;

        let target_spec = TargetSpecification {
            triple: target_triple.to_string(),
            platform: Platform::Linux, // Default platform for now
            architecture: Architecture::X86_64, // Default architecture for now
            requires_zig: matches!(optimization_level, OptimizationLevel::Release),
            compilation_strategy: crate::compilation::capabilities::CompilationStrategy::ZigBuild,
        };

        Ok(target_spec)
    }

    /// Get target info for a given target triple
    pub fn get_target_info(&self, target_triple: &str) -> Option<&TargetInfo> {
        self.supported_targets.get(target_triple)
    }

    /// Check if zigbuild is supported for a target
    pub fn is_zigbuild_supported(&self, target_triple: &str) -> bool {
        self.supported_targets
            .get(target_triple)
            .map(|info| info.zigbuild_supported)
            .unwrap_or(false)
    }

    /// Get all supported target triples
    pub fn get_supported_targets(&self) -> Vec<String> {
        self.supported_targets.keys().cloned().collect()
    }

    /// Convert platform enum to target triples
    pub fn get_targets_for_platform(&self, platform: &Platform) -> Vec<String> {
        self.supported_targets
            .values()
            .filter(|info| info.platform == *platform)
            .map(|info| info.target_triple.clone())
            .collect()
    }

    /// Create optimized target spec for deployment
    pub fn create_deployment_target_spec(
        &self,
        target_triple: &str,
        optimize_for_size: bool,
    ) -> Result<TargetSpecification, TargetDetectionError> {
        let optimization_level = if optimize_for_size {
            OptimizationLevel::MinSizeRelease
        } else {
            OptimizationLevel::Release
        };

        self.create_target_spec(target_triple, optimization_level)
    }

    fn get_default_target_cpu(&self, target_triple: &str) -> Option<String> {
        match target_triple {
            "aarch64-apple-darwin" => Some("apple-m1".to_string()),
            "x86_64-apple-darwin" => Some("haswell".to_string()), // Safe default for Intel Macs
            "x86_64-unknown-linux-gnu" => Some("x86-64".to_string()),
            "aarch64-unknown-linux-gnu" => Some("generic".to_string()),
            _ => None,
        }
    }
}

impl Default for TargetDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Utilities for target platform detection
pub mod utils {
    use super::*;

    /// Get the optimal target for localhost testing
    pub fn get_localhost_target() -> Result<String> {
        let detector = TargetDetector::new();
        Ok(detector.detect_host_target()?)
    }

    /// Check if current host can cross-compile to target
    pub fn can_cross_compile_to(target_triple: &str) -> bool {
        let detector = TargetDetector::new();
        detector.is_zigbuild_supported(target_triple) || which::which("cargo-zigbuild").is_ok()
    }

    /// Get recommended targets for a platform
    pub fn get_recommended_targets(platform: &Platform) -> Vec<String> {
        let detector = TargetDetector::new();
        detector.get_targets_for_platform(platform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_detection() {
        let detector = TargetDetector::new();
        let host_target = detector.detect_host_target();
        assert!(host_target.is_ok());
    }

    #[test]
    fn test_localhost_target_spec() {
        let detector = TargetDetector::new();
        let target_spec = detector.create_localhost_target_spec();
        assert!(target_spec.is_ok());
    }

    #[test]
    fn test_supported_targets() {
        let detector = TargetDetector::new();
        let targets = detector.get_supported_targets();
        assert!(!targets.is_empty());
        assert!(targets.contains(&"aarch64-apple-darwin".to_string()));
    }

    #[test]
    fn test_platform_targets() {
        let detector = TargetDetector::new();
        let macos_targets = detector.get_targets_for_platform(&Platform::MacOS);
        assert!(macos_targets.contains(&"aarch64-apple-darwin".to_string()));
        assert!(macos_targets.contains(&"x86_64-apple-darwin".to_string()));
    }

    #[test]
    fn test_zigbuild_support() {
        let detector = TargetDetector::new();
        assert!(detector.is_zigbuild_supported("aarch64-apple-darwin"));
        assert!(!detector.is_zigbuild_supported("x86_64-pc-windows-msvc"));
    }
}
