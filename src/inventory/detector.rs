use crate::inventory::error::DetectionError;
use crate::types::inventory::{ConnectionMethod, InventoryHost};
use std::collections::HashMap;

/// Architecture detection and target mapping
#[derive(Debug, Clone)]
pub struct ArchitectureDetector {
    pub target_mappings: HashMap<String, String>,
    pub platform_mappings: HashMap<String, String>,
}

impl ArchitectureDetector {
    pub fn new() -> Self {
        let mut target_mappings = HashMap::new();
        let mut platform_mappings = HashMap::new();

        // Common architecture mappings
        target_mappings.insert(
            "x86_64-linux".to_string(),
            "x86_64-unknown-linux-gnu".to_string(),
        );
        target_mappings.insert(
            "aarch64-linux".to_string(),
            "aarch64-unknown-linux-gnu".to_string(),
        );
        target_mappings.insert(
            "x86_64-darwin".to_string(),
            "x86_64-apple-darwin".to_string(),
        );
        target_mappings.insert(
            "aarch64-darwin".to_string(),
            "aarch64-apple-darwin".to_string(),
        );
        target_mappings.insert(
            "x86_64-windows".to_string(),
            "x86_64-pc-windows-msvc".to_string(),
        );

        // Platform to architecture mappings
        platform_mappings.insert(
            "debian-x86_64".to_string(),
            "x86_64-unknown-linux-gnu".to_string(),
        );
        platform_mappings.insert(
            "ubuntu-x86_64".to_string(),
            "x86_64-unknown-linux-gnu".to_string(),
        );
        platform_mappings.insert(
            "centos-x86_64".to_string(),
            "x86_64-unknown-linux-gnu".to_string(),
        );
        platform_mappings.insert(
            "redhat-x86_64".to_string(),
            "x86_64-unknown-linux-gnu".to_string(),
        );
        platform_mappings.insert(
            "fedora-x86_64".to_string(),
            "x86_64-unknown-linux-gnu".to_string(),
        );
        platform_mappings.insert(
            "alpine-x86_64".to_string(),
            "x86_64-unknown-linux-musl".to_string(),
        );

        Self {
            target_mappings,
            platform_mappings,
        }
    }

    pub fn detect_target_triple(&self, host: &InventoryHost) -> Option<String> {
        // Check explicit target_triple variable
        if let Some(triple) = &host.target_triple {
            return Some(triple.clone());
        }

        // Check architecture and platform variables
        if let (Some(arch), Some(platform)) = (
            host.variables.get("ansible_architecture"),
            host.variables.get("ansible_os_family"),
        ) {
            let arch_str = arch.as_str()?;
            let platform_str = platform.as_str()?;
            return self.map_platform_to_triple(platform_str, arch_str);
        }

        // Check for explicit architecture and operating_system
        if let (Some(arch), Some(os)) = (&host.architecture, &host.operating_system) {
            return self.map_platform_to_triple(os, arch);
        }

        // Fallback to connection-based detection
        match host.connection.method {
            ConnectionMethod::Local => Some(self.detect_local_triple()),
            ConnectionMethod::Ssh => self.probe_ssh_architecture(host),
            ConnectionMethod::WinRm => Some("x86_64-pc-windows-msvc".to_string()),
            _ => None,
        }
    }

    pub fn map_platform_to_triple(&self, platform: &str, arch: &str) -> Option<String> {
        let key = format!("{}-{}", platform.to_lowercase(), arch);
        if let Some(triple) = self.platform_mappings.get(&key) {
            return Some(triple.clone());
        }

        // Fallback to general mappings
        match (platform.to_lowercase().as_str(), arch) {
            ("debian" | "ubuntu" | "redhat" | "centos" | "fedora", "x86_64") => {
                Some("x86_64-unknown-linux-gnu".to_string())
            }
            ("debian" | "ubuntu" | "redhat" | "centos" | "fedora", "aarch64") => {
                Some("aarch64-unknown-linux-gnu".to_string())
            }
            ("alpine", "x86_64") => Some("x86_64-unknown-linux-musl".to_string()),
            ("alpine", "aarch64") => Some("aarch64-unknown-linux-musl".to_string()),
            ("darwin", "x86_64") => Some("x86_64-apple-darwin".to_string()),
            ("darwin", "arm64" | "aarch64") => Some("aarch64-apple-darwin".to_string()),
            ("windows", "amd64" | "x86_64") => Some("x86_64-pc-windows-msvc".to_string()),
            _ => None,
        }
    }

    pub fn probe_host_architecture(&self, host: &InventoryHost) -> Result<String, DetectionError> {
        match host.connection.method {
            ConnectionMethod::Ssh => {
                self.probe_ssh_architecture(host)
                    .ok_or_else(|| DetectionError::DetectionFailed {
                        reason: "SSH probe failed".to_string(),
                    })
            }
            ConnectionMethod::Local => Ok(self.detect_local_triple()),
            ConnectionMethod::WinRm => Ok("x86_64-pc-windows-msvc".to_string()),
            _ => Err(DetectionError::UnsupportedTarget {
                target: format!("{:?}", host.connection.method),
            }),
        }
    }

    fn detect_local_triple(&self) -> String {
        // Use the current platform's target triple
        #[cfg(target_arch = "x86_64")]
        #[cfg(target_os = "linux")]
        {
            "x86_64-unknown-linux-gnu".to_string()
        }
        #[cfg(target_arch = "aarch64")]
        #[cfg(target_os = "linux")]
        {
            "aarch64-unknown-linux-gnu".to_string()
        }
        #[cfg(target_arch = "x86_64")]
        #[cfg(target_os = "macos")]
        {
            "x86_64-apple-darwin".to_string()
        }
        #[cfg(target_arch = "aarch64")]
        #[cfg(target_os = "macos")]
        {
            "aarch64-apple-darwin".to_string()
        }
        #[cfg(target_arch = "x86_64")]
        #[cfg(target_os = "windows")]
        {
            "x86_64-pc-windows-msvc".to_string()
        }
        #[cfg(not(any(
            all(target_arch = "x86_64", target_os = "linux"),
            all(target_arch = "aarch64", target_os = "linux"),
            all(target_arch = "x86_64", target_os = "macos"),
            all(target_arch = "aarch64", target_os = "macos"),
            all(target_arch = "x86_64", target_os = "windows")
        )))]
        {
            "unknown".to_string()
        }
    }

    fn probe_ssh_architecture(&self, _host: &InventoryHost) -> Option<String> {
        // For now, return a default. In a full implementation, this would
        // actually SSH to the host and run architecture detection commands
        // TODO: Implement actual SSH probing
        Some("x86_64-unknown-linux-gnu".to_string())
    }
}

impl Default for ArchitectureDetector {
    fn default() -> Self {
        Self::new()
    }
}
