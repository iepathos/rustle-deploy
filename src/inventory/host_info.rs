use crate::inventory::error::ProbeError;
use crate::types::inventory::{ConnectionMethod, HostInfo, InventoryHost};

pub struct HostInfoProber;

impl HostInfoProber {
    pub fn new() -> Self {
        Self
    }

    pub fn probe_host_info(&self, host: &InventoryHost) -> Result<HostInfo, ProbeError> {
        match &host.connection.method {
            ConnectionMethod::Local => self.probe_local_info(),
            ConnectionMethod::Ssh => self.probe_ssh_info(host),
            ConnectionMethod::WinRm => self.probe_winrm_info(host),
            _ => Err(ProbeError::ConnectionFailed {
                host: host.name.clone(),
            }),
        }
    }

    fn probe_local_info(&self) -> Result<HostInfo, ProbeError> {
        Ok(HostInfo {
            architecture: self.detect_local_arch(),
            operating_system: self.detect_local_os(),
            platform: self.detect_local_platform(),
            kernel_version: self.detect_local_kernel(),
            target_triple: self.detect_local_target_triple(),
            capabilities: self.detect_local_capabilities(),
        })
    }

    fn probe_ssh_info(&self, _host: &InventoryHost) -> Result<HostInfo, ProbeError> {
        // For now, return a default implementation
        // In a full implementation, this would SSH to the host and run commands
        Ok(HostInfo {
            architecture: "x86_64".to_string(),
            operating_system: "Linux".to_string(),
            platform: "linux".to_string(),
            kernel_version: "unknown".to_string(),
            target_triple: "x86_64-unknown-linux-gnu".to_string(),
            capabilities: vec!["ssh".to_string()],
        })
    }

    fn probe_winrm_info(&self, _host: &InventoryHost) -> Result<HostInfo, ProbeError> {
        // For now, return a default implementation
        // In a full implementation, this would use WinRM to connect and gather info
        Ok(HostInfo {
            architecture: "x86_64".to_string(),
            operating_system: "Windows".to_string(),
            platform: "windows".to_string(),
            kernel_version: "unknown".to_string(),
            target_triple: "x86_64-pc-windows-msvc".to_string(),
            capabilities: vec!["winrm".to_string()],
        })
    }

    fn detect_local_arch(&self) -> String {
        std::env::consts::ARCH.to_string()
    }

    fn detect_local_os(&self) -> String {
        std::env::consts::OS.to_string()
    }

    fn detect_local_platform(&self) -> String {
        match std::env::consts::OS {
            "linux" => "linux".to_string(),
            "macos" => "darwin".to_string(),
            "windows" => "windows".to_string(),
            other => other.to_string(),
        }
    }

    fn detect_local_kernel(&self) -> String {
        // This would typically use system calls to get the kernel version
        // For now, return a placeholder
        "unknown".to_string()
    }

    fn detect_local_target_triple(&self) -> String {
        match (std::env::consts::ARCH, std::env::consts::OS) {
            ("x86_64", "linux") => "x86_64-unknown-linux-gnu".to_string(),
            ("aarch64", "linux") => "aarch64-unknown-linux-gnu".to_string(),
            ("x86_64", "macos") => "x86_64-apple-darwin".to_string(),
            ("aarch64", "macos") => "aarch64-apple-darwin".to_string(),
            ("x86_64", "windows") => "x86_64-pc-windows-msvc".to_string(),
            (arch, os) => format!("{arch}-unknown-{os}"),
        }
    }

    fn detect_local_capabilities(&self) -> Vec<String> {
        let mut capabilities = vec!["local".to_string()];

        // Check for common tools
        if self.command_exists("ssh") {
            capabilities.push("ssh".to_string());
        }
        if self.command_exists("podman") {
            capabilities.push("podman".to_string());
        }

        capabilities
    }

    fn command_exists(&self, command: &str) -> bool {
        std::process::Command::new("which")
            .arg(command)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

impl Default for HostInfoProber {
    fn default() -> Self {
        Self::new()
    }
}
