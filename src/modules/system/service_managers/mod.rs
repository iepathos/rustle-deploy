//! Service manager implementations

use crate::modules::error::ServiceManagerError;
use async_trait::async_trait;

#[async_trait]
pub trait ServiceManager: Send + Sync {
    async fn query_service(&self, name: &str) -> Result<ServiceStatus, ServiceManagerError>;
    async fn start_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError>;
    async fn stop_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError>;
    async fn restart_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError>;
    async fn reload_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError>;
    async fn enable_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError>;
    async fn disable_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError>;
}

#[derive(Debug, Clone)]
pub struct ServiceStatus {
    pub running: bool,
    pub enabled: Option<bool>,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct ServiceResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

// Platform-specific service managers
pub mod init;
pub mod launchd;
pub mod systemd;
pub mod windows;

pub use init::InitServiceManager;
pub use launchd::LaunchdServiceManager;
pub use systemd::SystemdServiceManager;
pub use windows::WindowsServiceManager;
