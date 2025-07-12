//! Windows-specific fact collection

use super::PlatformFactCollector;
use crate::modules::system::facts::FactError;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;

pub struct WindowsFactCollector;

#[async_trait]
impl PlatformFactCollector for WindowsFactCollector {
    async fn collect_platform_facts(
        &self,
    ) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();

        facts.insert("ansible_system".to_string(), json!("Win32NT"));
        facts.insert("ansible_distribution".to_string(), json!("Windows"));
        facts.insert("ansible_os_family".to_string(), json!("Windows"));

        // Get Windows version using PowerShell
        if let Ok(output) = tokio::process::Command::new("powershell")
            .arg("-Command")
            .arg("Get-WmiObject -Class Win32_OperatingSystem | Select-Object Version, Caption | ConvertTo-Json")
            .output()
            .await
        {
            if output.status.success() {
                let json_output = String::from_utf8_lossy(&output.stdout);
                if let Ok(os_info) = serde_json::from_str::<serde_json::Value>(&json_output) {
                    if let Some(version) = os_info.get("Version").and_then(|v| v.as_str()) {
                        facts.insert("ansible_distribution_version".to_string(), json!(version));
                    }
                    if let Some(caption) = os_info.get("Caption").and_then(|v| v.as_str()) {
                        facts.insert("ansible_distribution_release".to_string(), json!(caption));
                    }
                }
            }
        }

        // Get kernel version (same as OS version on Windows)
        if let Some(version) = facts.get("ansible_distribution_version") {
            facts.insert("ansible_kernel".to_string(), version.clone());
            facts.insert("ansible_kernel_version".to_string(), version.clone());
        }

        // Get machine architecture
        facts.insert("ansible_machine".to_string(), json!(std::env::consts::ARCH));

        Ok(facts)
    }

    async fn collect_virtualization_facts(
        &self,
    ) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();

        let virt_type = self.detect_virtualization().await;
        facts.insert("ansible_virtualization_type".to_string(), json!(virt_type));

        let virt_role = if virt_type == "physical" {
            "host"
        } else {
            "guest"
        };
        facts.insert("ansible_virtualization_role".to_string(), json!(virt_role));

        Ok(facts)
    }
}

impl WindowsFactCollector {
    async fn detect_virtualization(&self) -> String {
        // Check for virtualization using WMI
        if let Ok(output) = tokio::process::Command::new("powershell")
            .arg("-Command")
            .arg("Get-WmiObject -Class Win32_ComputerSystem | Select-Object Model, Manufacturer | ConvertTo-Json")
            .output()
            .await
        {
            if output.status.success() {
                let json_output = String::from_utf8_lossy(&output.stdout);
                if let Ok(system_info) = serde_json::from_str::<serde_json::Value>(&json_output) {
                    let model = system_info.get("Model")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_lowercase();
                    let manufacturer = system_info.get("Manufacturer")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_lowercase();

                    if model.contains("vmware") || manufacturer.contains("vmware") {
                        return "vmware".to_string();
                    }
                    if model.contains("virtualbox") || manufacturer.contains("innotek") {
                        return "virtualbox".to_string();
                    }
                    if model.contains("virtual machine") || manufacturer.contains("microsoft") {
                        return "hyperv".to_string();
                    }
                    if model.contains("parallels") {
                        return "parallels".to_string();
                    }}
            }
        }

        "physical".to_string()
    }
}
