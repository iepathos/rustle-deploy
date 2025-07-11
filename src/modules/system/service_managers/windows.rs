//! Windows service manager

use crate::modules::{
    error::ServiceManagerError,
    system::service_managers::{ServiceManager, ServiceResult, ServiceStatus},
};
use async_trait::async_trait;
use tokio::process::Command;

pub struct WindowsServiceManager;

impl Default for WindowsServiceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsServiceManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ServiceManager for WindowsServiceManager {
    async fn query_service(&self, name: &str) -> Result<ServiceStatus, ServiceManagerError> {
        let output = Command::new("sc").args(["query", name]).output().await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let running = stdout.contains("RUNNING");
        let status = if running {
            "running".to_string()
        } else if stdout.contains("STOPPED") {
            "stopped".to_string()
        } else {
            "unknown".to_string()
        };

        // Check if service is set to auto-start
        let config_output = Command::new("sc").args(["qc", name]).output().await?;

        let config_stdout = String::from_utf8_lossy(&config_output.stdout);
        let enabled = if config_stdout.contains("AUTO_START") {
            Some(true)
        } else if config_stdout.contains("DEMAND_START") {
            Some(false)
        } else {
            None
        };

        Ok(ServiceStatus {
            running,
            enabled,
            status,
        })
    }

    async fn start_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError> {
        let output = Command::new("sc").args(["start", name]).output().await?;

        Ok(ServiceResult {
            success: output.status.success(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    async fn stop_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError> {
        let output = Command::new("sc").args(["stop", name]).output().await?;

        Ok(ServiceResult {
            success: output.status.success(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    async fn restart_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError> {
        // Windows doesn't have a direct restart, so stop then start
        let _stop_result = self.stop_service(name).await?;
        // Wait a moment for the service to fully stop
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        self.start_service(name).await
    }

    async fn reload_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError> {
        // Windows services don't typically support reload, so restart
        self.restart_service(name).await
    }

    async fn enable_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError> {
        let output = Command::new("sc")
            .args(["config", name, "start=", "auto"])
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
        let output = Command::new("sc")
            .args(["config", name, "start=", "demand"])
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
