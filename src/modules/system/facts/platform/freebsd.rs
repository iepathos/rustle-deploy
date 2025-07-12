//! FreeBSD-specific fact collection

use super::PlatformFactCollector;
use crate::modules::system::facts::FactError;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;

pub struct FreeBSDFactCollector;

#[async_trait]
impl PlatformFactCollector for FreeBSDFactCollector {
    async fn collect_platform_facts(
        &self,
    ) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();

        facts.insert("ansible_system".to_string(), json!("FreeBSD"));
        facts.insert("ansible_distribution".to_string(), json!("FreeBSD"));
        facts.insert("ansible_os_family".to_string(), json!("FreeBSD"));

        // Get FreeBSD version using uname
        if let Ok(output) = tokio::process::Command::new("uname")
            .arg("-r")
            .output()
            .await
        {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                facts.insert(
                    "ansible_distribution_version".to_string(),
                    json!(version.clone()),
                );
                facts.insert("ansible_kernel".to_string(), json!(version));
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

        // FreeBSD release information
        if let Ok(output) = tokio::process::Command::new("uname")
            .arg("-p")
            .output()
            .await
        {
            if output.status.success() {
                let release = String::from_utf8_lossy(&output.stdout).trim().to_string();
                facts.insert("ansible_distribution_release".to_string(), json!(release));
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

impl FreeBSDFactCollector {
    async fn detect_virtualization(&self) -> String {
        // Check for virtualization using sysctl
        if let Ok(output) = tokio::process::Command::new("sysctl")
            .arg("-n")
            .arg("kern.vm_guest")
            .output()
            .await
        {
            if output.status.success() {
                let vm_guest = String::from_utf8_lossy(&output.stdout).trim();
                match vm_guest {
                    "vmware" => return "vmware".to_string(),
                    "xen" => return "xen".to_string(),
                    "hv" => return "hyperv".to_string(),
                    "kvm" => return "kvm".to_string(),
                    "bhyve" => return "bhyve".to_string(),
                    "none" => {} // Continue checking other methods
                    _ => return vm_guest.to_string(),
                }
            }
        }

        // Check dmesg for virtualization indicators
        if let Ok(output) = tokio::process::Command::new("dmesg").output().await {
            if output.status.success() {
                let dmesg_output = String::from_utf8_lossy(&output.stdout).to_lowercase();
                if dmesg_output.contains("vmware") {
                    return "vmware".to_string();
                }
                if dmesg_output.contains("virtualbox") {
                    return "virtualbox".to_string();
                }
                if dmesg_output.contains("bhyve") {
                    return "bhyve".to_string();
                }
                if dmesg_output.contains("xen") {
                    return "xen".to_string();
                }
            }
        }

        "physical".to_string()
    }
}
