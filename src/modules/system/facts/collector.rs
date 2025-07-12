//! Main facts collector implementation

use super::custom::CustomFactsLoader;
use super::hardware::HardwareCollector;
use super::network::NetworkCollector;
use super::platform::PlatformFactCollector;
use super::{cache::FactCache, FactCategory, FactError, SystemFacts};
use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Duration;

#[async_trait]
pub trait FactCollector: Send + Sync {
    async fn collect_facts(&self, subset: &[FactCategory]) -> Result<SystemFacts, FactError>;
}

pub struct SystemFactCollector {
    platform_collector: Box<dyn PlatformFactCollector>,
    hardware_collector: HardwareCollector,
    network_collector: NetworkCollector,
    custom_facts_loader: CustomFactsLoader,
    cache: FactCache,
    timeout: Duration,
}

impl SystemFactCollector {
    pub fn new() -> Self {
        let platform_collector = Self::create_platform_collector();

        Self {
            platform_collector,
            hardware_collector: HardwareCollector::new(),
            network_collector: NetworkCollector::new(),
            custom_facts_loader: CustomFactsLoader::new(vec![]),
            cache: FactCache::new(Duration::from_secs(3600)),
            timeout: Duration::from_secs(30),
        }
    }

    pub fn with_custom_fact_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.custom_facts_loader = CustomFactsLoader::new(paths);
        self
    }

    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache = FactCache::new(ttl);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    fn create_platform_collector() -> Box<dyn PlatformFactCollector> {
        #[cfg(target_os = "linux")]
        {
            Box::new(super::platform::linux::LinuxFactCollector)
        }
        #[cfg(target_os = "macos")]
        {
            Box::new(super::platform::macos::MacOSFactCollector)
        }
        #[cfg(target_os = "windows")]
        {
            Box::new(super::platform::windows::WindowsFactCollector)
        }
        #[cfg(target_os = "freebsd")]
        {
            Box::new(super::platform::freebsd::FreeBSDFactCollector)
        }
        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "windows",
            target_os = "freebsd"
        )))]
        {
            Box::new(super::platform::unix_common::UnixFactCollector)
        }
    }
}

#[async_trait]
impl FactCollector for SystemFactCollector {
    async fn collect_facts(&self, subset: &[FactCategory]) -> Result<SystemFacts, FactError> {
        let hostname = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "localhost".to_string());

        // Check cache first
        if let Some(cached) = self.cache.get_facts(&hostname).await {
            return Ok(cached);
        }

        let mut facts = SystemFacts::default();

        // Collect facts based on requested categories
        for category in subset {
            match category {
                FactCategory::All | FactCategory::Default => {
                    // Collect all facts
                    self.collect_platform_facts(&mut facts).await?;
                    self.collect_hardware_facts(&mut facts).await?;
                    self.collect_network_facts(&mut facts).await?;
                    self.collect_virtualization_facts(&mut facts).await?;
                    self.collect_environment_facts(&mut facts).await?;
                }
                FactCategory::Platform | FactCategory::Distribution => {
                    self.collect_platform_facts(&mut facts).await?;
                }
                FactCategory::Hardware => {
                    self.collect_hardware_facts(&mut facts).await?;
                }
                FactCategory::Network | FactCategory::Interfaces => {
                    self.collect_network_facts(&mut facts).await?;
                }
                FactCategory::Virtual => {
                    self.collect_virtualization_facts(&mut facts).await?;
                }
                FactCategory::Env => {
                    self.collect_environment_facts(&mut facts).await?;
                }
                _ => {
                    // Skip unsupported categories for now
                }
            }
        }

        // Load custom facts
        let custom_facts = self.custom_facts_loader.load_custom_facts().await?;
        facts.ansible_local = custom_facts;

        // Cache the results
        self.cache.cache_facts(&hostname, facts.clone(), None).await;

        Ok(facts)
    }
}

impl SystemFactCollector {
    async fn collect_platform_facts(&self, facts: &mut SystemFacts) -> Result<(), FactError> {
        let platform_facts = self.platform_collector.collect_platform_facts().await?;

        for (key, value) in platform_facts {
            match key.as_str() {
                "ansible_system" => {
                    if let Some(system) = value.as_str() {
                        facts.ansible_system = system.to_string();
                    }
                }
                "ansible_os_family" => {
                    if let Some(family) = value.as_str() {
                        facts.ansible_os_family = family.to_string();
                    }
                }
                "ansible_distribution" => {
                    if let Some(dist) = value.as_str() {
                        facts.ansible_distribution = dist.to_string();
                    }
                }
                "ansible_distribution_version" => {
                    if let Some(version) = value.as_str() {
                        facts.ansible_distribution_version = version.to_string();
                    }
                }
                "ansible_distribution_release" => {
                    if let Some(release) = value.as_str() {
                        facts.ansible_distribution_release = release.to_string();
                    }
                }
                "ansible_kernel" => {
                    if let Some(kernel) = value.as_str() {
                        facts.ansible_kernel = kernel.to_string();
                    }
                }
                "ansible_kernel_version" => {
                    if let Some(version) = value.as_str() {
                        facts.ansible_kernel_version = version.to_string();
                    }
                }
                "ansible_machine" => {
                    if let Some(machine) = value.as_str() {
                        facts.ansible_machine = machine.to_string();
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn collect_hardware_facts(&self, facts: &mut SystemFacts) -> Result<(), FactError> {
        let hw_facts = self.hardware_collector.collect_hardware_facts().await?;

        for (key, value) in hw_facts {
            match key.as_str() {
                "ansible_processor" => {
                    if let Some(processor_array) = value.as_array() {
                        facts.ansible_processor = processor_array
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                    }
                }
                "ansible_processor_count" => {
                    if let Some(count) = value.as_u64() {
                        facts.ansible_processor_count = count as u32;
                    }
                }
                "ansible_processor_cores" => {
                    if let Some(cores) = value.as_u64() {
                        facts.ansible_processor_cores = cores as u32;
                    }
                }
                "ansible_processor_vcpus" => {
                    if let Some(vcpus) = value.as_u64() {
                        facts.ansible_processor_vcpus = vcpus as u32;
                    }
                }
                "ansible_memtotal_mb" => {
                    if let Some(mem) = value.as_u64() {
                        facts.ansible_memtotal_mb = mem;
                    }
                }
                "ansible_memfree_mb" => {
                    if let Some(mem) = value.as_u64() {
                        facts.ansible_memfree_mb = mem;
                    }
                }
                "ansible_swaptotal_mb" => {
                    if let Some(swap) = value.as_u64() {
                        facts.ansible_swaptotal_mb = swap;
                    }
                }
                "ansible_swapfree_mb" => {
                    if let Some(swap) = value.as_u64() {
                        facts.ansible_swapfree_mb = swap;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn collect_network_facts(&self, facts: &mut SystemFacts) -> Result<(), FactError> {
        let net_facts = self.network_collector.collect_network_facts().await?;

        for (key, value) in net_facts {
            match key.as_str() {
                "ansible_hostname" => {
                    if let Some(hostname) = value.as_str() {
                        facts.ansible_hostname = hostname.to_string();
                    }
                }
                "ansible_fqdn" => {
                    if let Some(fqdn) = value.as_str() {
                        facts.ansible_fqdn = fqdn.to_string();
                    }
                }
                "ansible_domain" => {
                    if let Some(domain) = value.as_str() {
                        facts.ansible_domain = domain.to_string();
                    }
                }
                "ansible_all_ipv4_addresses" => {
                    if let Some(addresses) = value.as_array() {
                        facts.ansible_all_ipv4_addresses = addresses
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                    }
                }
                "ansible_all_ipv6_addresses" => {
                    if let Some(addresses) = value.as_array() {
                        facts.ansible_all_ipv6_addresses = addresses
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                    }
                }
                "ansible_interfaces" => {
                    if let Some(interfaces) = value.as_array() {
                        facts.ansible_interfaces = interfaces
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn collect_virtualization_facts(&self, facts: &mut SystemFacts) -> Result<(), FactError> {
        let virt_facts = self
            .platform_collector
            .collect_virtualization_facts()
            .await?;

        for (key, value) in virt_facts {
            match key.as_str() {
                "ansible_virtualization_type" => {
                    if let Some(virt_type) = value.as_str() {
                        facts.ansible_virtualization_type = virt_type.to_string();
                    }
                }
                "ansible_virtualization_role" => {
                    if let Some(virt_role) = value.as_str() {
                        facts.ansible_virtualization_role = virt_role.to_string();
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn collect_environment_facts(&self, facts: &mut SystemFacts) -> Result<(), FactError> {
        // Collect user information
        if let Some(username) = std::env::var("USER")
            .ok()
            .or_else(|| std::env::var("USERNAME").ok())
        {
            facts.ansible_user_id = username;
        }

        if let Some(home) = std::env::var("HOME")
            .ok()
            .or_else(|| std::env::var("USERPROFILE").ok())
        {
            facts.ansible_user_dir = home;
        }

        if let Ok(shell) = std::env::var("SHELL") {
            facts.ansible_user_shell = shell;
        }

        // Detect package manager
        facts.ansible_pkg_mgr = self.detect_package_manager();

        // Detect service manager
        facts.ansible_service_mgr = self.detect_service_manager();

        // Check for Python
        facts.ansible_python_version = self.detect_python_version().await;

        Ok(())
    }

    fn detect_package_manager(&self) -> String {
        #[cfg(target_os = "linux")]
        {
            if std::path::Path::new("/usr/bin/apt").exists()
                || std::path::Path::new("/usr/bin/apt-get").exists()
            {
                return "apt".to_string();
            }
            if std::path::Path::new("/usr/bin/yum").exists() {
                return "yum".to_string();
            }
            if std::path::Path::new("/usr/bin/dnf").exists() {
                return "dnf".to_string();
            }
            if std::path::Path::new("/usr/bin/pacman").exists() {
                return "pacman".to_string();
            }
            if std::path::Path::new("/usr/bin/zypper").exists() {
                return "zypper".to_string();
            }
        }

        #[cfg(target_os = "macos")]
        {
            if std::path::Path::new("/usr/local/bin/brew").exists()
                || std::path::Path::new("/opt/homebrew/bin/brew").exists()
            {
                return "brew".to_string();
            }
            if std::path::Path::new("/opt/local/bin/port").exists() {
                return "macports".to_string();
            }
        }

        #[cfg(target_os = "windows")]
        {
            return "chocolatey".to_string(); // Assume chocolatey as default on Windows
        }

        #[cfg(target_os = "freebsd")]
        {
            if std::path::Path::new("/usr/local/sbin/pkg").exists() {
                return "pkg".to_string();
            }
        }

        "unknown".to_string()
    }

    fn detect_service_manager(&self) -> String {
        #[cfg(target_os = "linux")]
        {
            if std::path::Path::new("/run/systemd/system").exists() {
                return "systemd".to_string();
            }
            if std::path::Path::new("/sbin/init").exists() {
                return "sysvinit".to_string();
            }
            return "unknown".to_string();
        }

        #[cfg(target_os = "macos")]
        {
            "launchd".to_string()
        }

        #[cfg(target_os = "windows")]
        {
            return "win32_service".to_string();
        }

        #[cfg(target_os = "freebsd")]
        {
            return "rc".to_string();
        }

        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "windows",
            target_os = "freebsd"
        )))]
        {
            "unknown".to_string()
        }
    }

    async fn detect_python_version(&self) -> String {
        let python_commands = ["python3", "python", "python2"];

        for cmd in &python_commands {
            if let Ok(output) = tokio::process::Command::new(cmd)
                .arg("--version")
                .output()
                .await
            {
                if output.status.success() {
                    let version_str = String::from_utf8_lossy(&output.stdout);
                    if let Some(version) = version_str.split_whitespace().nth(1) {
                        return version.to_string();
                    }
                }
            }
        }

        "Not found".to_string()
    }
}

impl Default for SystemFactCollector {
    fn default() -> Self {
        Self::new()
    }
}
