//! Init script service manager for legacy Linux systems

use crate::modules::{
    error::ServiceManagerError,
    system::service_managers::{ServiceManager, ServiceResult, ServiceStatus},
};
use async_trait::async_trait;
use tokio::process::Command;

pub struct InitServiceManager;

impl Default for InitServiceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl InitServiceManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ServiceManager for InitServiceManager {
    async fn query_service(&self, name: &str) -> Result<ServiceStatus, ServiceManagerError> {
        let output = Command::new("service")
            .args([name, "status"])
            .output()
            .await?;

        let running = output.status.success();
        let status = String::from_utf8_lossy(&output.stdout).trim().to_string();

        Ok(ServiceStatus {
            running,
            enabled: None, // Init scripts don't have a standard way to check enabled status
            status,
        })
    }

    async fn start_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError> {
        let output = Command::new("service")
            .args([name, "start"])
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
        let output = Command::new("service")
            .args([name, "stop"])
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
        let output = Command::new("service")
            .args([name, "restart"])
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
        let output = Command::new("service")
            .args([name, "reload"])
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
        // Try chkconfig first, then update-rc.d
        let chkconfig_output = Command::new("chkconfig").args([name, "on"]).output().await;

        if let Ok(output) = chkconfig_output {
            if output.status.success() {
                return Ok(ServiceResult {
                    success: true,
                    exit_code: 0,
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                });
            }
        }

        // Try update-rc.d
        let update_rc_output = Command::new("update-rc.d")
            .args([name, "enable"])
            .output()
            .await?;

        Ok(ServiceResult {
            success: update_rc_output.status.success(),
            exit_code: update_rc_output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&update_rc_output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&update_rc_output.stderr).to_string(),
        })
    }

    async fn disable_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError> {
        // Try chkconfig first, then update-rc.d
        let chkconfig_output = Command::new("chkconfig").args([name, "off"]).output().await;

        if let Ok(output) = chkconfig_output {
            if output.status.success() {
                return Ok(ServiceResult {
                    success: true,
                    exit_code: 0,
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                });
            }
        }

        // Try update-rc.d
        let update_rc_output = Command::new("update-rc.d")
            .args([name, "disable"])
            .output()
            .await?;

        Ok(ServiceResult {
            success: update_rc_output.status.success(),
            exit_code: update_rc_output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&update_rc_output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&update_rc_output.stderr).to_string(),
        })
    }
}
