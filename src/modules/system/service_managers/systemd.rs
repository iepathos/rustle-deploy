//! Systemd service manager for Linux systems

use crate::modules::{
    error::ServiceManagerError,
    system::service_managers::{ServiceManager, ServiceResult, ServiceStatus},
};
use async_trait::async_trait;
use tokio::process::Command;

pub struct SystemdServiceManager;

impl SystemdServiceManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ServiceManager for SystemdServiceManager {
    async fn query_service(&self, name: &str) -> Result<ServiceStatus, ServiceManagerError> {
        let status_output = Command::new("systemctl")
            .args(&["is-active", name])
            .output()
            .await?;

        let enabled_output = Command::new("systemctl")
            .args(&["is-enabled", name])
            .output()
            .await?;

        let running = status_output.status.success()
            && String::from_utf8_lossy(&status_output.stdout).trim() == "active";

        let enabled = if enabled_output.status.success() {
            let enabled_stdout = String::from_utf8_lossy(&enabled_output.stdout);
            let enabled_str = enabled_stdout.trim();
            Some(enabled_str == "enabled")
        } else {
            None
        };

        let status = String::from_utf8_lossy(&status_output.stdout)
            .trim()
            .to_string();

        Ok(ServiceStatus {
            running,
            enabled,
            status,
        })
    }

    async fn start_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError> {
        let output = Command::new("systemctl")
            .args(&["start", name])
            .output()
            .await?;

        Ok(ServiceResult {
            success: output.status.success(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    async fn stop_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError> {
        let output = Command::new("systemctl")
            .args(&["stop", name])
            .output()
            .await?;

        Ok(ServiceResult {
            success: output.status.success(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    async fn restart_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError> {
        let output = Command::new("systemctl")
            .args(&["restart", name])
            .output()
            .await?;

        Ok(ServiceResult {
            success: output.status.success(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    async fn reload_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError> {
        let output = Command::new("systemctl")
            .args(&["reload", name])
            .output()
            .await?;

        Ok(ServiceResult {
            success: output.status.success(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    async fn enable_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError> {
        let output = Command::new("systemctl")
            .args(&["enable", name])
            .output()
            .await?;

        Ok(ServiceResult {
            success: output.status.success(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    async fn disable_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError> {
        let output = Command::new("systemctl")
            .args(&["disable", name])
            .output()
            .await?;

        Ok(ServiceResult {
            success: output.status.success(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}
