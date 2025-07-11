//! Launchd service manager for macOS

use crate::modules::{
    error::ServiceManagerError,
    system::service_managers::{ServiceManager, ServiceResult, ServiceStatus},
};
use async_trait::async_trait;
use tokio::process::Command;

pub struct LaunchdServiceManager;

impl LaunchdServiceManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ServiceManager for LaunchdServiceManager {
    async fn query_service(&self, name: &str) -> Result<ServiceStatus, ServiceManagerError> {
        let output = Command::new("launchctl")
            .args(&["list", name])
            .output()
            .await?;

        let running = output.status.success();
        let status = if running {
            "loaded".to_string()
        } else {
            "unloaded".to_string()
        };

        Ok(ServiceStatus {
            running,
            enabled: Some(running), // In launchd, loaded generally means enabled
            status,
        })
    }

    async fn start_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError> {
        let output = Command::new("launchctl")
            .args(&[
                "load",
                "-w",
                &format!("/Library/LaunchDaemons/{}.plist", name),
            ])
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
        let output = Command::new("launchctl")
            .args(&[
                "unload",
                "-w",
                &format!("/Library/LaunchDaemons/{}.plist", name),
            ])
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
        // For launchd, restart is unload then load
        let _stop_result = self.stop_service(name).await?;
        self.start_service(name).await
    }

    async fn reload_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError> {
        // Launchd doesn't have a direct reload, so restart
        self.restart_service(name).await
    }

    async fn enable_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError> {
        // Enable is the same as start in launchd
        self.start_service(name).await
    }

    async fn disable_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError> {
        // Disable is the same as stop in launchd
        self.stop_service(name).await
    }
}
