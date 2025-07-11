//! Chocolatey package manager for Windows

use crate::modules::{
    error::PackageManagerError,
    system::package_managers::{Package, PackageManager, PackageResult, PackageState},
};
use async_trait::async_trait;
use tokio::process::Command;

pub struct ChocolateyPackageManager;

impl Default for ChocolateyPackageManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ChocolateyPackageManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PackageManager for ChocolateyPackageManager {
    async fn query_package(&self, name: &str) -> Result<PackageState, PackageManagerError> {
        let output = Command::new("choco")
            .args(["list", "--local-only", name])
            .output()
            .await?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout
                .lines()
                .any(|line| line.to_lowercase().contains(&name.to_lowercase()))
            {
                Ok(PackageState::Present)
            } else {
                Ok(PackageState::Absent)
            }
        } else {
            Ok(PackageState::Absent)
        }
    }

    async fn install_package(&self, name: &str) -> Result<PackageResult, PackageManagerError> {
        let output = Command::new("choco")
            .args(["install", name, "-y"])
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        Ok(PackageResult {
            success: output.status.success(),
            exit_code,
            stdout,
            stderr,
            message: if output.status.success() {
                Some(format!("Package {name} installed successfully"))
            } else {
                Some(format!("Failed to install package {name}"))
            },
        })
    }

    async fn remove_package(&self, name: &str) -> Result<PackageResult, PackageManagerError> {
        let output = Command::new("choco")
            .args(["uninstall", name, "-y"])
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        Ok(PackageResult {
            success: output.status.success(),
            exit_code,
            stdout,
            stderr,
            message: if output.status.success() {
                Some(format!("Package {name} removed successfully"))
            } else {
                Some(format!("Failed to remove package {name}"))
            },
        })
    }

    async fn list_packages(&self) -> Result<Vec<Package>, PackageManagerError> {
        let output = Command::new("choco")
            .args(["list", "--local-only"])
            .output()
            .await?;

        if !output.status.success() {
            return Err(PackageManagerError::OperationFailed {
                error: "Failed to list packages".to_string(),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            if line.contains(" ") && !line.starts_with("Chocolatey") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    packages.push(Package {
                        name: parts[0].to_string(),
                        version: parts[1].to_string(),
                        description: None,
                    });
                }
            }
        }

        Ok(packages)
    }
}
