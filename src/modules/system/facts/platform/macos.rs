//! macOS-specific fact collection

use super::PlatformFactCollector;
use crate::modules::system::facts::FactError;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;

pub struct MacOSFactCollector;

#[async_trait]
impl PlatformFactCollector for MacOSFactCollector {
    async fn collect_platform_facts(
        &self,
    ) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();

        facts.insert("ansible_system".to_string(), json!("Darwin"));
        facts.insert("ansible_distribution".to_string(), json!("MacOSX"));
        facts.insert("ansible_os_family".to_string(), json!("Darwin"));

        // Get macOS version using sw_vers
        if let Ok(output) = tokio::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .await
        {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                facts.insert("ansible_distribution_version".to_string(), json!(version));

                // Determine release name based on version
                let release_name = self.determine_release_name(&version);
                facts.insert(
                    "ansible_distribution_release".to_string(),
                    json!(release_name),
                );
            }
        }

        // Get kernel version
        if let Ok(output) = tokio::process::Command::new("uname")
            .arg("-r")
            .output()
            .await
        {
            if output.status.success() {
                let kernel = String::from_utf8_lossy(&output.stdout).trim().to_string();
                facts.insert("ansible_kernel".to_string(), json!(kernel));
            }
        }

        // Get full kernel version
        if let Ok(output) = tokio::process::Command::new("uname")
            .arg("-v")
            .output()
            .await
        {
            if output.status.success() {
                let kernel_version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                facts.insert("ansible_kernel_version".to_string(), json!(kernel_version));
            }
        }

        // Get machine type
        if let Ok(output) = tokio::process::Command::new("uname")
            .arg("-m")
            .output()
            .await
        {
            if output.status.success() {
                let machine = String::from_utf8_lossy(&output.stdout).trim().to_string();
                facts.insert("ansible_machine".to_string(), json!(machine));
            }
        }

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

impl MacOSFactCollector {
    fn determine_release_name(&self, version: &str) -> String {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() >= 2 {
            let major: i32 = parts[0].parse().unwrap_or(0);
            let minor: i32 = parts[1].parse().unwrap_or(0);

            match (major, minor) {
                (14, _) => "Sonoma".to_string(),
                (13, _) => "Ventura".to_string(),
                (12, _) => "Monterey".to_string(),
                (11, _) => "Big Sur".to_string(),
                (10, 15) => "Catalina".to_string(),
                (10, 14) => "Mojave".to_string(),
                (10, 13) => "High Sierra".to_string(),
                (10, 12) => "Sierra".to_string(),
                (10, 11) => "El Capitan".to_string(),
                (10, 10) => "Yosemite".to_string(),
                _ => "Unknown".to_string(),
            }
        } else {
            "Unknown".to_string()
        }
    }

    async fn detect_virtualization(&self) -> String {
        // Check for VMware
        if let Ok(output) = tokio::process::Command::new("system_profiler")
            .arg("SPHardwareDataType")
            .output()
            .await
        {
            if output.status.success() {
                let hardware_info = String::from_utf8_lossy(&output.stdout);
                if hardware_info.to_lowercase().contains("vmware") {
                    return "vmware".to_string();
                }
                if hardware_info.to_lowercase().contains("virtualbox") {
                    return "virtualbox".to_string();
                }
                if hardware_info.to_lowercase().contains("parallels") {
                    return "parallels".to_string();
                }
            }
        }

        // Check for Apple Silicon vs Intel for additional context
        if let Ok(output) = tokio::process::Command::new("uname")
            .arg("-m")
            .output()
            .await
        {
            if output.status.success() {
                let machine_output = String::from_utf8_lossy(&output.stdout);
                let machine = machine_output.trim();
                if machine == "arm64" {
                    // Check if running under Rosetta
                    if let Ok(output) = tokio::process::Command::new("sysctl")
                        .arg("-n")
                        .arg("sysctl.proc_translated")
                        .output()
                        .await
                    {
                        if output.status.success() {
                            let translated_output = String::from_utf8_lossy(&output.stdout);
                            let translated = translated_output.trim();
                            if translated == "1" {
                                return "rosetta".to_string();
                            }
                        }
                    }
                }
            }
        }

        "physical".to_string()
    }
}
