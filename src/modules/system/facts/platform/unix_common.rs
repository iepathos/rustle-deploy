//! Common Unix fact collection for unsupported platforms

use super::PlatformFactCollector;
use crate::modules::system::facts::FactError;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;

pub struct UnixFactCollector;

#[async_trait]
impl PlatformFactCollector for UnixFactCollector {
    async fn collect_platform_facts(
        &self,
    ) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();

        // Try to determine system using uname
        if let Ok(output) = tokio::process::Command::new("uname")
            .arg("-s")
            .output()
            .await
        {
            if output.status.success() {
                let system = String::from_utf8_lossy(&output.stdout).trim().to_string();
                facts.insert("ansible_system".to_string(), json!(system));
                facts.insert("ansible_distribution".to_string(), json!(system.clone()));
                facts.insert("ansible_os_family".to_string(), json!(system));
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

        // Default to physical for unknown platforms
        facts.insert("ansible_virtualization_type".to_string(), json!("physical"));
        facts.insert("ansible_virtualization_role".to_string(), json!("host"));

        Ok(facts)
    }
}
