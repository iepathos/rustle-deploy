use crate::deploy::{DeployError, Result};
use std::collections::HashMap;
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct CrossCompiler {
    supported_targets: HashMap<String, TargetInfo>,
    toolchain_manager: ToolchainManager,
}

#[derive(Debug, Clone)]
pub struct TargetInfo {
    pub triple: String,
    pub display_name: String,
    pub requires_toolchain: bool,
    pub toolchain_name: Option<String>,
    pub default_features: Vec<String>,
    pub binary_extension: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ToolchainManager {
    installed_toolchains: HashMap<String, ToolchainInfo>,
}

#[derive(Debug, Clone)]
pub struct ToolchainInfo {
    pub name: String,
    pub version: String,
    pub installed: bool,
    pub targets: Vec<String>,
}

impl CrossCompiler {
    pub fn new() -> Self {
        let mut supported_targets = HashMap::new();

        // Add commonly supported targets
        supported_targets.insert(
            "x86_64-unknown-linux-gnu".to_string(),
            TargetInfo {
                triple: "x86_64-unknown-linux-gnu".to_string(),
                display_name: "Linux x86_64".to_string(),
                requires_toolchain: false,
                toolchain_name: None,
                default_features: vec![],
                binary_extension: None,
            },
        );

        supported_targets.insert(
            "aarch64-unknown-linux-gnu".to_string(),
            TargetInfo {
                triple: "aarch64-unknown-linux-gnu".to_string(),
                display_name: "Linux ARM64".to_string(),
                requires_toolchain: true,
                toolchain_name: Some("aarch64-linux-gnu".to_string()),
                default_features: vec![],
                binary_extension: None,
            },
        );

        supported_targets.insert(
            "x86_64-apple-darwin".to_string(),
            TargetInfo {
                triple: "x86_64-apple-darwin".to_string(),
                display_name: "macOS x86_64".to_string(),
                requires_toolchain: true,
                toolchain_name: Some("x86_64-apple-darwin".to_string()),
                default_features: vec![],
                binary_extension: None,
            },
        );

        supported_targets.insert(
            "aarch64-apple-darwin".to_string(),
            TargetInfo {
                triple: "aarch64-apple-darwin".to_string(),
                display_name: "macOS ARM64".to_string(),
                requires_toolchain: true,
                toolchain_name: Some("aarch64-apple-darwin".to_string()),
                default_features: vec![],
                binary_extension: None,
            },
        );

        supported_targets.insert(
            "x86_64-pc-windows-gnu".to_string(),
            TargetInfo {
                triple: "x86_64-pc-windows-gnu".to_string(),
                display_name: "Windows x86_64".to_string(),
                requires_toolchain: true,
                toolchain_name: Some("x86_64-w64-mingw32".to_string()),
                default_features: vec![],
                binary_extension: Some(".exe".to_string()),
            },
        );

        Self {
            supported_targets,
            toolchain_manager: ToolchainManager::new(),
        }
    }

    pub fn is_target_supported(&self, target_triple: &str) -> bool {
        self.supported_targets.contains_key(target_triple)
    }

    pub fn get_target_info(&self, target_triple: &str) -> Option<&TargetInfo> {
        self.supported_targets.get(target_triple)
    }

    pub fn list_supported_targets(&self) -> Vec<&TargetInfo> {
        self.supported_targets.values().collect()
    }

    pub async fn ensure_target_available(&self, target_triple: &str) -> Result<()> {
        info!("Ensuring target is available: {}", target_triple);

        let target_info =
            self.get_target_info(target_triple)
                .ok_or_else(|| DeployError::UnsupportedTarget {
                    target: target_triple.to_string(),
                })?;

        // Check if rustup target is installed
        if !self.is_rustup_target_installed(target_triple).await? {
            info!("Installing rustup target: {}", target_triple);
            self.install_rustup_target(target_triple).await?;
        }

        // Check if cross-compilation toolchain is needed
        if target_info.requires_toolchain {
            if let Some(toolchain_name) = &target_info.toolchain_name {
                if !self
                    .toolchain_manager
                    .is_toolchain_available(toolchain_name)
                {
                    warn!(
                        "Cross-compilation toolchain not available: {}",
                        toolchain_name
                    );
                    info!("Consider installing the required toolchain or using Docker for cross-compilation");
                }
            }
        }

        debug!("Target {} is ready for compilation", target_triple);
        Ok(())
    }

    pub fn detect_host_target(&self) -> String {
        // Detect current host target triple
        std::env::consts::ARCH.to_string()
            + "-"
            + if cfg!(target_os = "linux") {
                "unknown-linux-gnu"
            } else if cfg!(target_os = "macos") {
                "apple-darwin"
            } else if cfg!(target_os = "windows") {
                "pc-windows-msvc"
            } else {
                "unknown"
            }
    }

    pub fn suggest_targets_for_inventory(&self, inventory_hosts: &[String]) -> Vec<String> {
        // Simple heuristic for target detection based on hostnames/IPs
        // In a real implementation, this would query the hosts for their architecture
        let mut suggested_targets = Vec::new();

        for host in inventory_hosts {
            // Default to Linux x86_64 for most hosts
            let target = if host.contains("arm") || host.contains("aarch64") {
                "aarch64-unknown-linux-gnu"
            } else if host.contains("win") || host.contains("windows") {
                "x86_64-pc-windows-gnu"
            } else if host.contains("mac") || host.contains("darwin") {
                "x86_64-apple-darwin"
            } else {
                "x86_64-unknown-linux-gnu"
            };

            if !suggested_targets.contains(&target.to_string()) {
                suggested_targets.push(target.to_string());
            }
        }

        suggested_targets
    }

    // Private helper methods

    async fn is_rustup_target_installed(&self, target_triple: &str) -> Result<bool> {
        use tokio::process::Command;

        let output = Command::new("rustup")
            .args(["target", "list", "--installed"])
            .output()
            .await
            .map_err(|e| DeployError::Configuration(format!("Failed to run rustup: {e}")))?;

        if !output.status.success() {
            return Err(DeployError::Configuration(
                "rustup command failed".to_string(),
            ));
        }

        let installed_targets = String::from_utf8_lossy(&output.stdout);
        Ok(installed_targets
            .lines()
            .any(|line| line.trim() == target_triple))
    }

    async fn install_rustup_target(&self, target_triple: &str) -> Result<()> {
        use tokio::process::Command;

        info!("Installing rustup target: {}", target_triple);

        let output = Command::new("rustup")
            .args(["target", "add", target_triple])
            .output()
            .await
            .map_err(|e| DeployError::Configuration(format!("Failed to run rustup: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DeployError::Configuration(format!(
                "Failed to install target {target_triple}: {stderr}"
            )));
        }

        info!("Successfully installed target: {}", target_triple);
        Ok(())
    }
}

impl Default for ToolchainManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolchainManager {
    pub fn new() -> Self {
        Self {
            installed_toolchains: HashMap::new(),
        }
    }

    pub fn is_toolchain_available(&self, toolchain_name: &str) -> bool {
        // Simple check - in a real implementation this would actually verify
        // that the cross-compilation toolchain is installed and functional
        self.installed_toolchains.contains_key(toolchain_name)
    }

    pub async fn detect_installed_toolchains(&mut self) -> Result<()> {
        // Detect common cross-compilation toolchains
        // This is a simplified implementation

        // Check for common toolchain packages
        let common_toolchains = vec![
            ("aarch64-linux-gnu", "gcc-aarch64-linux-gnu"),
            ("arm-linux-gnueabihf", "gcc-arm-linux-gnueabihf"),
            ("x86_64-w64-mingw32", "gcc-mingw-w64-x86-64"),
        ];

        for (name, package) in common_toolchains {
            if self.is_package_installed(package).await {
                self.installed_toolchains.insert(
                    name.to_string(),
                    ToolchainInfo {
                        name: name.to_string(),
                        version: "unknown".to_string(),
                        installed: true,
                        targets: vec![],
                    },
                );
            }
        }

        Ok(())
    }

    async fn is_package_installed(&self, package_name: &str) -> bool {
        // Check if a package is installed (Linux/apt example)
        use tokio::process::Command;

        let output = Command::new("dpkg")
            .args(["-l", package_name])
            .output()
            .await;

        if let Ok(output) = output {
            output.status.success()
        } else {
            false
        }
    }
}

impl Default for CrossCompiler {
    fn default() -> Self {
        Self::new()
    }
}
