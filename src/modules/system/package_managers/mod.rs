//! Package manager implementations

use crate::modules::error::PackageManagerError;
use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum PackageState {
    Present,
    Absent,
}

#[async_trait]
pub trait PackageManager: Send + Sync {
    async fn query_package(&self, name: &str) -> Result<PackageState, PackageManagerError>;
    async fn install_package(&self, name: &str) -> Result<PackageResult, PackageManagerError>;
    async fn remove_package(&self, name: &str) -> Result<PackageResult, PackageManagerError>;
    async fn list_packages(&self) -> Result<Vec<Package>, PackageManagerError>;
}

#[derive(Debug, Clone)]
pub struct PackageResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
}

// Platform-specific package managers
pub mod apt;
pub mod brew;
pub mod chocolatey;
pub mod dnf;
pub mod yum;

pub use apt::AptPackageManager;
pub use brew::BrewPackageManager;
pub use chocolatey::ChocolateyPackageManager;
pub use dnf::DnfPackageManager;
pub use yum::YumPackageManager;
