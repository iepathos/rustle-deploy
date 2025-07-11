//! APT package manager for Debian/Ubuntu systems

use crate::modules::{
    error::PackageManagerError,
    system::package_managers::{Package, PackageManager, PackageResult, PackageState},
};
use async_trait::async_trait;
use tokio::process::Command;

pub struct AptPackageManager;

impl Default for AptPackageManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AptPackageManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PackageManager for AptPackageManager {
    async fn query_package(&self, name: &str) -> Result<PackageState, PackageManagerError> {
        let output = Command::new("dpkg").args(["-l", name]).output().await?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Check if package is installed (status 'ii')
            if stdout
                .lines()
                .any(|line| line.starts_with("ii") && line.contains(name))
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
        let output = Command::new("apt-get")
            .args(["install", "-y", name])
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
        let output = Command::new("apt-get")
            .args(["remove", "-y", name])
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
        let output = Command::new("dpkg").args(["-l"]).output().await?;

        if !output.status.success() {
            return Err(PackageManagerError::OperationFailed {
                error: "Failed to list packages".to_string(),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            if line.starts_with("ii") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    packages.push(Package {
                        name: parts[1].to_string(),
                        version: parts[2].to_string(),
                        description: if parts.len() > 3 {
                            Some(parts[3..].join(" "))
                        } else {
                            None
                        },
                    });
                }
            }
        }

        Ok(packages)
    }
}
