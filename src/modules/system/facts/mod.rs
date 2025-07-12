//! System facts collection framework

pub mod cache;
pub mod collector;
pub mod custom;
pub mod hardware;
pub mod network;
pub mod platform;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemFacts {
    // Platform and OS information
    pub ansible_system: String,       // "Linux", "Darwin", "Windows"
    pub ansible_os_family: String,    // "RedHat", "Debian", "Windows"
    pub ansible_distribution: String, // "Ubuntu", "CentOS", "macOS"
    pub ansible_distribution_version: String, // "20.04", "8.2", "12.1"
    pub ansible_distribution_release: String, // "focal", "ootpa"
    pub ansible_architecture: String, // "x86_64", "aarch64", "i386"
    pub ansible_machine: String,      // Hardware platform identifier
    pub ansible_kernel: String,       // Kernel version
    pub ansible_kernel_version: String, // Full kernel version string

    // Hardware information
    pub ansible_processor: Vec<String>,          // CPU information
    pub ansible_processor_count: u32,            // Number of physical CPUs
    pub ansible_processor_cores: u32,            // Total CPU cores
    pub ansible_processor_threads_per_core: u32, // Threads per core
    pub ansible_processor_vcpus: u32,            // Total virtual CPUs
    pub ansible_memtotal_mb: u64,                // Total memory in MB
    pub ansible_memfree_mb: u64,                 // Free memory in MB
    pub ansible_swaptotal_mb: u64,               // Total swap in MB
    pub ansible_swapfree_mb: u64,                // Free swap in MB

    // Network information
    pub ansible_all_ipv4_addresses: Vec<String>, // All IPv4 addresses
    pub ansible_all_ipv6_addresses: Vec<String>, // All IPv6 addresses
    pub ansible_default_ipv4: Option<DefaultInterface>, // Default IPv4 interface
    pub ansible_default_ipv6: Option<DefaultInterface>, // Default IPv6 interface
    pub ansible_hostname: String,                // Short hostname
    pub ansible_fqdn: String,                    // Fully qualified domain name
    pub ansible_domain: String,                  // DNS domain
    pub ansible_interfaces: Vec<String>,         // List of network interfaces

    // Per-interface details (dynamic keys)
    #[serde(flatten)]
    pub interface_facts: HashMap<String, InterfaceFacts>,

    // Environment and user information
    pub ansible_user_id: String,              // Current user
    pub ansible_user_uid: u32,                // User ID
    pub ansible_user_gid: u32,                // Group ID
    pub ansible_user_gecos: String,           // User GECOS field
    pub ansible_user_dir: String,             // User home directory
    pub ansible_user_shell: String,           // User shell
    pub ansible_env: HashMap<String, String>, // Environment variables

    // System paths and configuration
    pub ansible_pkg_mgr: String, // Package manager (apt, yum, brew, etc.)
    pub ansible_service_mgr: String, // Service manager (systemd, launchd, etc.)
    pub ansible_python_version: String, // Python version (if available)
    pub ansible_system_capabilities: Vec<String>, // System capabilities

    // Virtualization information
    pub ansible_virtualization_type: String, // "kvm", "vmware", "docker", "physical"
    pub ansible_virtualization_role: String, // "guest", "host", "NA"

    // Custom facts
    pub ansible_local: HashMap<String, serde_json::Value>, // Local custom facts
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultInterface {
    pub interface: String,
    pub address: String,
    pub gateway: String,
    pub network: String,
    pub netmask: String,
    pub broadcast: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceFacts {
    pub device: String,
    pub active: bool,
    #[serde(rename = "type")]
    pub type_: String, // "ether", "loopback", "bridge"
    pub macaddress: Option<String>,
    pub mtu: Option<u32>,
    pub ipv4: Option<InterfaceIPv4>,
    pub ipv6: Vec<InterfaceIPv6>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceIPv4 {
    pub address: String,
    pub netmask: String,
    pub network: String,
    pub broadcast: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceIPv6 {
    pub address: String,
    pub prefix: u8,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FactCategory {
    All,
    Hardware,
    Network,
    Virtual,
    Ohai,   // Chef Ohai-style facts
    Facter, // Puppet Facter-style facts
    Platform,
    Distribution,
    Cmdline,
    Python,
    Env,
    Interfaces,
    Default, // Essential facts only
}

impl Default for SystemFacts {
    fn default() -> Self {
        Self {
            ansible_system: "Unknown".to_string(),
            ansible_os_family: "Unknown".to_string(),
            ansible_distribution: "Unknown".to_string(),
            ansible_distribution_version: "Unknown".to_string(),
            ansible_distribution_release: "Unknown".to_string(),
            ansible_architecture: std::env::consts::ARCH.to_string(),
            ansible_machine: "Unknown".to_string(),
            ansible_kernel: "Unknown".to_string(),
            ansible_kernel_version: "Unknown".to_string(),
            ansible_processor: Vec::new(),
            ansible_processor_count: 0,
            ansible_processor_cores: 0,
            ansible_processor_threads_per_core: 1,
            ansible_processor_vcpus: 0,
            ansible_memtotal_mb: 0,
            ansible_memfree_mb: 0,
            ansible_swaptotal_mb: 0,
            ansible_swapfree_mb: 0,
            ansible_all_ipv4_addresses: Vec::new(),
            ansible_all_ipv6_addresses: Vec::new(),
            ansible_default_ipv4: None,
            ansible_default_ipv6: None,
            ansible_hostname: hostname::get()
                .ok()
                .and_then(|h| h.into_string().ok())
                .unwrap_or_else(|| "unknown".to_string()),
            ansible_fqdn: "unknown".to_string(),
            ansible_domain: "unknown".to_string(),
            ansible_interfaces: Vec::new(),
            interface_facts: HashMap::new(),
            ansible_user_id: "unknown".to_string(),
            ansible_user_uid: 0,
            ansible_user_gid: 0,
            ansible_user_gecos: "Unknown".to_string(),
            ansible_user_dir: "/".to_string(),
            ansible_user_shell: "/bin/sh".to_string(),
            ansible_env: std::env::vars().collect(),
            ansible_pkg_mgr: "unknown".to_string(),
            ansible_service_mgr: "unknown".to_string(),
            ansible_python_version: "Not found".to_string(),
            ansible_system_capabilities: Vec::new(),
            ansible_virtualization_type: "unknown".to_string(),
            ansible_virtualization_role: "unknown".to_string(),
            ansible_local: HashMap::new(),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum FactError {
    #[error("System command failed: {command}")]
    CommandFailed { command: String },

    #[error("Failed to parse system information: {0}")]
    ParseError(String),

    #[error("Permission denied accessing: {path}")]
    PermissionDenied { path: String },

    #[error("Timeout collecting facts after {timeout}s")]
    Timeout { timeout: u64 },

    #[error("Network interface detection failed: {0}")]
    NetworkError(String),

    #[error("Custom fact loading failed: {path}")]
    CustomFactError { path: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}
