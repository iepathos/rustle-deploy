//! Linux-specific fact collection

use super::PlatformFactCollector;
use crate::modules::system::facts::FactError;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use tokio::fs;

pub struct LinuxFactCollector;

#[async_trait]
impl PlatformFactCollector for LinuxFactCollector {
    async fn collect_platform_facts(
        &self,
    ) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();

        facts.insert("ansible_system".to_string(), json!("Linux"));

        // Read /etc/os-release
        if let Ok(os_release) = fs::read_to_string("/etc/os-release").await {
            facts.extend(self.parse_os_release(&os_release)?);
        }

        // Fallback to /etc/lsb-release
        if !facts.contains_key("ansible_distribution") {
            if let Ok(lsb_release) = fs::read_to_string("/etc/lsb-release").await {
                facts.extend(self.parse_lsb_release(&lsb_release)?);
            }
        }

        // Read /proc/version for kernel information
        if let Ok(version) = fs::read_to_string("/proc/version").await {
            facts.extend(self.parse_proc_version(&version)?);
        }

        // Read kernel version from uname
        if let Ok(output) = tokio::process::Command::new("uname")
            .args(&["-r"])
            .output()
            .await
        {
            if output.status.success() {
                let kernel = String::from_utf8_lossy(&output.stdout).trim().to_string();
                facts.insert("ansible_kernel".to_string(), json!(kernel));
            }
        }

        // Get machine information
        if let Ok(output) = tokio::process::Command::new("uname")
            .args(&["-m"])
            .output()
            .await
        {
            if output.status.success() {
                let machine = String::from_utf8_lossy(&output.stdout).trim().to_string();
                facts.insert("ansible_machine".to_string(), json!(machine));
            }
        }

        // Set OS family based on distribution
        if let Some(distribution) = facts.get("ansible_distribution") {
            if let Some(dist_str) = distribution.as_str() {
                let os_family = self.determine_os_family(dist_str);
                facts.insert("ansible_os_family".to_string(), json!(os_family));
            }
        }

        Ok(facts)
    }

    async fn collect_virtualization_facts(
        &self,
    ) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();

        // Check for virtualization
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

impl LinuxFactCollector {
    fn parse_os_release(
        &self,
        content: &str,
    ) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();

        for line in content.lines() {
            if let Some((key, value)) = line.split_once('=') {
                let value = value.trim_matches('"');
                match key {
                    "ID" => {
                        facts.insert(
                            "ansible_distribution".to_string(),
                            json!(self.normalize_distribution(value)),
                        );
                    }
                    "VERSION_ID" => {
                        facts.insert("ansible_distribution_version".to_string(), json!(value));
                    }
                    "VERSION_CODENAME" => {
                        facts.insert("ansible_distribution_release".to_string(), json!(value));
                    }
                    "PRETTY_NAME" => {
                        if !facts.contains_key("ansible_distribution") {
                            if let Some(dist_name) = value.split_whitespace().next() {
                                facts.insert(
                                    "ansible_distribution".to_string(),
                                    json!(self.normalize_distribution(dist_name)),
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(facts)
    }

    fn parse_lsb_release(
        &self,
        content: &str,
    ) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();

        for line in content.lines() {
            if let Some((key, value)) = line.split_once('=') {
                let value = value.trim_matches('"');
                match key {
                    "DISTRIB_ID" => {
                        facts.insert(
                            "ansible_distribution".to_string(),
                            json!(self.normalize_distribution(value)),
                        );
                    }
                    "DISTRIB_RELEASE" => {
                        facts.insert("ansible_distribution_version".to_string(), json!(value));
                    }
                    "DISTRIB_CODENAME" => {
                        facts.insert("ansible_distribution_release".to_string(), json!(value));
                    }
                    _ => {}
                }
            }
        }

        Ok(facts)
    }

    fn parse_proc_version(
        &self,
        content: &str,
    ) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();

        // Extract full kernel version
        facts.insert("ansible_kernel_version".to_string(), json!(content.trim()));

        Ok(facts)
    }

    fn normalize_distribution(&self, dist: &str) -> String {
        match dist.to_lowercase().as_str() {
            "ubuntu" => "Ubuntu".to_string(),
            "debian" => "Debian".to_string(),
            "centos" => "CentOS".to_string(),
            "rhel" | "redhat" => "RedHat".to_string(),
            "fedora" => "Fedora".to_string(),
            "opensuse" | "suse" => "SUSE".to_string(),
            "arch" => "Archlinux".to_string(),
            "alpine" => "Alpine".to_string(),
            _ => dist.to_string(),
        }
    }

    fn determine_os_family(&self, distribution: &str) -> String {
        match distribution.to_lowercase().as_str() {
            "ubuntu" | "debian" | "mint" => "Debian".to_string(),
            "centos" | "redhat" | "rhel" | "fedora" | "amazon" => "RedHat".to_string(),
            "opensuse" | "suse" | "sles" => "Suse".to_string(),
            "archlinux" | "arch" | "manjaro" => "Archlinux".to_string(),
            "alpine" => "Alpine".to_string(),
            "gentoo" => "Gentoo".to_string(),
            _ => "Linux".to_string(),
        }
    }

    async fn detect_virtualization(&self) -> String {
        // Check systemd-detect-virt if available
        if let Ok(output) = tokio::process::Command::new("systemd-detect-virt")
            .output()
            .await
        {
            if output.status.success() {
                let virt_type = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if virt_type != "none" {
                    return virt_type;
                }
            }
        }

        // Check DMI information
        if let Ok(product_name) = fs::read_to_string("/sys/class/dmi/id/product_name").await {
            let product_name = product_name.trim().to_lowercase();
            if product_name.contains("vmware") {
                return "vmware".to_string();
            }
            if product_name.contains("virtualbox") {
                return "virtualbox".to_string();
            }
            if product_name.contains("kvm") {
                return "kvm".to_string();
            }
            if product_name.contains("qemu") {
                return "qemu".to_string();
            }
        }

        // Check for container environments
        if fs::metadata("/.dockerenv").await.is_ok() {
            return "docker".to_string();
        }

        if let Ok(cgroup) = fs::read_to_string("/proc/1/cgroup").await {
            if cgroup.contains("docker") {
                return "docker".to_string();
            }
            if cgroup.contains("lxc") {
                return "lxc".to_string();
            }
        }

        // Check CPU flags for virtualization
        if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo").await {
            if cpuinfo.contains("hypervisor") {
                return "kvm".to_string(); // Generic hypervisor
            }
        }

        "physical".to_string()
    }
}
