use crate::runtime::FactsError;
use serde_json::json;
use std::collections::HashMap;
use std::process::Command;
use std::time::{Duration, Instant};

/// System facts collector
pub struct FactsCollector;

impl FactsCollector {
    pub fn collect_all_facts() -> Result<HashMap<String, serde_json::Value>, FactsError> {
        let mut facts = HashMap::new();

        facts.extend(Self::collect_system_facts()?);
        facts.extend(Self::collect_network_facts()?);
        facts.extend(Self::collect_platform_facts()?);
        facts.extend(Self::collect_hardware_facts()?);

        Ok(facts)
    }

    fn collect_system_facts() -> Result<HashMap<String, serde_json::Value>, FactsError> {
        let mut facts = HashMap::new();

        // Basic system information
        if let Ok(hostname) = hostname::get() {
            facts.insert(
                "ansible_hostname".to_string(),
                json!(hostname.to_string_lossy()),
            );
        }

        facts.insert("ansible_fqdn".to_string(), json!(Self::get_fqdn()?));
        facts.insert(
            "ansible_os_family".to_string(),
            json!(Self::get_os_family()),
        );
        facts.insert("ansible_system".to_string(), json!(std::env::consts::OS));
        facts.insert(
            "ansible_architecture".to_string(),
            json!(std::env::consts::ARCH),
        );

        // Kernel and version information
        if let Ok(uname) = Self::run_command("uname -r") {
            facts.insert("ansible_kernel".to_string(), json!(uname.trim()));
        }

        // Distribution information (Linux)
        if cfg!(target_os = "linux") {
            facts.extend(Self::collect_linux_distribution_facts()?);
        }

        Ok(facts)
    }

    fn collect_network_facts() -> Result<HashMap<String, serde_json::Value>, FactsError> {
        let mut facts = HashMap::new();

        // Get network interfaces
        let interfaces = Self::get_network_interfaces()?;
        facts.insert("ansible_interfaces".to_string(), json!(interfaces));

        // Get default gateway and routes
        if let Ok(default_ipv4) = Self::get_default_route() {
            facts.insert("ansible_default_ipv4".to_string(), json!(default_ipv4));
        }

        Ok(facts)
    }

    fn collect_platform_facts() -> Result<HashMap<String, serde_json::Value>, FactsError> {
        let mut facts = HashMap::new();

        facts.insert("ansible_machine".to_string(), json!(std::env::consts::ARCH));
        facts.insert(
            "ansible_processor".to_string(),
            json!(Self::get_processor_info()?),
        );
        facts.insert(
            "ansible_processor_count".to_string(),
            json!(num_cpus::get()),
        );
        facts.insert(
            "ansible_processor_cores".to_string(),
            json!(num_cpus::get_physical()),
        );

        Ok(facts)
    }

    fn collect_hardware_facts() -> Result<HashMap<String, serde_json::Value>, FactsError> {
        let mut facts = HashMap::new();

        // Memory information
        if let Ok(meminfo) = Self::get_memory_info() {
            facts.extend(meminfo);
        }

        // Disk information
        if let Ok(diskinfo) = Self::get_disk_info() {
            facts.extend(diskinfo);
        }

        Ok(facts)
    }

    fn collect_linux_distribution_facts() -> Result<HashMap<String, serde_json::Value>, FactsError>
    {
        let mut facts = HashMap::new();

        // Try to read /etc/os-release
        if let Ok(os_release) = std::fs::read_to_string("/etc/os-release") {
            for line in os_release.lines() {
                if let Some((key, value)) = line.split_once('=') {
                    let value = value.trim_matches('"');
                    match key {
                        "ID" => {
                            facts.insert("ansible_distribution".to_string(), json!(value));
                        }
                        "VERSION_ID" => {
                            facts.insert("ansible_distribution_version".to_string(), json!(value));
                        }
                        "VERSION_CODENAME" => {
                            facts.insert("ansible_distribution_release".to_string(), json!(value));
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(facts)
    }

    fn get_os_family() -> &'static str {
        match std::env::consts::OS {
            "linux" => "RedHat", // Could be more specific based on distribution
            "macos" => "Darwin",
            "windows" => "Windows",
            _ => "Unknown",
        }
    }

    fn get_fqdn() -> Result<String, FactsError> {
        // Try to get FQDN, fallback to hostname
        Self::run_command("hostname -f")
            .or_else(|_| Self::run_command("hostname"))
            .map(|s| s.trim().to_string())
    }

    fn get_processor_info() -> Result<String, FactsError> {
        if cfg!(target_os = "linux") {
            // Try to get CPU model from /proc/cpuinfo
            if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
                for line in cpuinfo.lines() {
                    if line.starts_with("model name") {
                        if let Some(model) = line.split(':').nth(1) {
                            return Ok(model.trim().to_string());
                        }
                    }
                }
            }
        }

        // Fallback to architecture
        Ok(std::env::consts::ARCH.to_string())
    }

    fn get_memory_info() -> Result<HashMap<String, serde_json::Value>, FactsError> {
        let mut facts = HashMap::new();

        if cfg!(target_os = "linux") {
            if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
                for line in meminfo.lines() {
                    if let Some((key, value)) = line.split_once(':') {
                        let value = value.trim();
                        match key {
                            "MemTotal" => {
                                if let Ok(kb) = value
                                    .split_whitespace()
                                    .next()
                                    .unwrap_or("0")
                                    .parse::<u64>()
                                {
                                    facts.insert(
                                        "ansible_memtotal_mb".to_string(),
                                        json!(kb / 1024),
                                    );
                                }
                            }
                            "MemFree" => {
                                if let Ok(kb) = value
                                    .split_whitespace()
                                    .next()
                                    .unwrap_or("0")
                                    .parse::<u64>()
                                {
                                    facts
                                        .insert("ansible_memfree_mb".to_string(), json!(kb / 1024));
                                }
                            }
                            "SwapTotal" => {
                                if let Ok(kb) = value
                                    .split_whitespace()
                                    .next()
                                    .unwrap_or("0")
                                    .parse::<u64>()
                                {
                                    facts.insert(
                                        "ansible_swaptotal_mb".to_string(),
                                        json!(kb / 1024),
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(facts)
    }

    fn get_disk_info() -> Result<HashMap<String, serde_json::Value>, FactsError> {
        let mut facts = HashMap::new();

        // This is a simplified implementation
        // In practice, you'd want to parse df output or use platform-specific APIs
        if let Ok(df_output) = Self::run_command("df -h /") {
            let lines: Vec<&str> = df_output.lines().collect();
            if lines.len() >= 2 {
                let fields: Vec<&str> = lines[1].split_whitespace().collect();
                if fields.len() >= 4 {
                    facts.insert("ansible_disk_total".to_string(), json!(fields[1]));
                    facts.insert("ansible_disk_used".to_string(), json!(fields[2]));
                    facts.insert("ansible_disk_available".to_string(), json!(fields[3]));
                }
            }
        }

        Ok(facts)
    }

    fn get_network_interfaces() -> Result<Vec<String>, FactsError> {
        let mut interfaces = Vec::new();

        if cfg!(target_os = "linux") {
            if let Ok(output) = Self::run_command("ip link show") {
                for line in output.lines() {
                    if let Some(interface) = line.split(':').nth(1) {
                        let interface = interface.trim().split('@').next().unwrap_or("").trim();
                        if !interface.is_empty() && interface != "lo" {
                            interfaces.push(interface.to_string());
                        }
                    }
                }
            }
        } else if cfg!(target_os = "macos") {
            if let Ok(output) = Self::run_command("ifconfig -l") {
                interfaces = output
                    .split_whitespace()
                    .filter(|&iface| iface != "lo0")
                    .map(|s| s.to_string())
                    .collect();
            }
        }

        Ok(interfaces)
    }

    fn get_default_route() -> Result<HashMap<String, serde_json::Value>, FactsError> {
        let mut route_info = HashMap::new();

        if cfg!(target_os = "linux") {
            if let Ok(output) = Self::run_command("ip route show default") {
                let fields: Vec<&str> = output.split_whitespace().collect();
                if fields.len() >= 3 {
                    route_info.insert("gateway".to_string(), json!(fields[2]));
                }
                if let Some(dev_index) = fields.iter().position(|&x| x == "dev") {
                    if fields.len() > dev_index + 1 {
                        route_info.insert("interface".to_string(), json!(fields[dev_index + 1]));
                    }
                }
            }
        }

        Ok(route_info)
    }

    fn run_command(cmd: &str) -> Result<String, FactsError> {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd").arg("/C").arg(cmd).output()
        } else {
            Command::new("sh").arg("-c").arg(cmd).output()
        }?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(FactsError::CommandFailed {
                command: cmd.to_string(),
                error: String::from_utf8_lossy(&output.stderr).to_string(),
            })
        }
    }
}

/// Cache for facts to avoid repeated collection
pub struct FactsCache {
    cache: HashMap<String, CachedFact>,
    ttl: Duration,
}

#[derive(Debug, Clone)]
struct CachedFact {
    value: serde_json::Value,
    collected_at: Instant,
}

impl FactsCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: HashMap::new(),
            ttl,
        }
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        if let Some(cached) = self.cache.get(key) {
            if cached.collected_at.elapsed() < self.ttl {
                return Some(&cached.value);
            }
        }
        None
    }

    pub fn set(&mut self, key: String, value: serde_json::Value) {
        self.cache.insert(
            key,
            CachedFact {
                value,
                collected_at: Instant::now(),
            },
        );
    }

    pub fn invalidate(&mut self, key: &str) {
        self.cache.remove(key);
    }

    pub fn clear_expired(&mut self) {
        let now = Instant::now();
        self.cache
            .retain(|_, cached| now.duration_since(cached.collected_at) < self.ttl);
    }

    pub fn get_all_facts(&self) -> HashMap<String, serde_json::Value> {
        let now = Instant::now();
        self.cache
            .iter()
            .filter(|(_, cached)| now.duration_since(cached.collected_at) < self.ttl)
            .map(|(key, cached)| (key.clone(), cached.value.clone()))
            .collect()
    }

    pub fn refresh_facts(&mut self) -> Result<(), FactsError> {
        let facts = FactsCollector::collect_all_facts()?;
        for (key, value) in facts {
            self.set(key, value);
        }
        Ok(())
    }
}
