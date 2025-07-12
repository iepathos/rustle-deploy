//! Hardware fact collection

use crate::modules::system::facts::FactError;
use serde_json::json;
use std::collections::HashMap;
#[cfg(target_os = "linux")]
use tokio::fs;

pub struct HardwareCollector;

impl Default for HardwareCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl HardwareCollector {
    pub fn new() -> Self {
        Self
    }

    pub async fn collect_hardware_facts(
        &self,
    ) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();

        // Collect CPU information
        facts.extend(self.collect_cpu_facts().await?);

        // Collect memory information
        facts.extend(self.collect_memory_facts().await?);

        Ok(facts)
    }

    async fn collect_cpu_facts(&self) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();

        #[cfg(target_os = "linux")]
        {
            if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo").await {
                facts.extend(self.parse_linux_cpuinfo(&cpuinfo)?);
            }
        }

        #[cfg(target_os = "macos")]
        {
            facts.extend(self.collect_macos_cpu_facts().await?);
        }

        #[cfg(target_os = "windows")]
        {
            facts.extend(self.collect_windows_cpu_facts().await?);
        }

        // Fallback: get logical CPU count
        if !facts.contains_key("ansible_processor_vcpus") {
            let vcpus = num_cpus::get();
            facts.insert("ansible_processor_vcpus".to_string(), json!(vcpus));
            facts.insert("ansible_processor_count".to_string(), json!(1));
            facts.insert("ansible_processor_cores".to_string(), json!(vcpus));
            facts.insert("ansible_processor_threads_per_core".to_string(), json!(1));
        }

        Ok(facts)
    }

    async fn collect_memory_facts(&self) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();

        #[cfg(target_os = "linux")]
        {
            if let Ok(meminfo) = fs::read_to_string("/proc/meminfo").await {
                facts.extend(self.parse_linux_meminfo(&meminfo)?);
            }
        }

        #[cfg(target_os = "macos")]
        {
            facts.extend(self.collect_macos_memory_facts().await?);
        }

        #[cfg(target_os = "windows")]
        {
            facts.extend(self.collect_windows_memory_facts().await?);
        }

        Ok(facts)
    }

    #[cfg(target_os = "linux")]
    fn parse_linux_cpuinfo(
        &self,
        cpuinfo: &str,
    ) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();
        let mut processors = Vec::new();
        let mut processor_count = 0;
        let mut cores_per_package = 0;
        let mut threads_per_core = 1;
        let mut packages = std::collections::HashSet::new();

        for line in cpuinfo.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "processor" => {
                        processor_count += 1;
                    }
                    "model name" => {
                        if !processors.contains(&value.to_string()) {
                            processors.push(value.to_string());
                        }
                    }
                    "physical id" => {
                        packages.insert(value.to_string());
                    }
                    "cpu cores" => {
                        if let Ok(cores) = value.parse::<u32>() {
                            cores_per_package = cores;
                        }
                    }
                    "siblings" => {
                        if let Ok(siblings) = value.parse::<u32>() {
                            if cores_per_package > 0 {
                                threads_per_core = siblings / cores_per_package;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        let physical_packages = packages.len().max(1) as u32;
        let total_cores = physical_packages * cores_per_package.max(1);

        facts.insert("ansible_processor".to_string(), json!(processors));
        facts.insert(
            "ansible_processor_count".to_string(),
            json!(physical_packages),
        );
        facts.insert("ansible_processor_cores".to_string(), json!(total_cores));
        facts.insert(
            "ansible_processor_threads_per_core".to_string(),
            json!(threads_per_core),
        );
        facts.insert(
            "ansible_processor_vcpus".to_string(),
            json!(processor_count),
        );

        Ok(facts)
    }

    #[cfg(target_os = "linux")]
    fn parse_linux_meminfo(
        &self,
        meminfo: &str,
    ) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();

        for line in meminfo.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                if let Some(kb_str) = value.split_whitespace().next() {
                    if let Ok(kb) = kb_str.parse::<u64>() {
                        let mb = kb / 1024;
                        match key {
                            "MemTotal" => {
                                facts.insert("ansible_memtotal_mb".to_string(), json!(mb));
                            }
                            "MemFree" => {
                                facts.insert("ansible_memfree_mb".to_string(), json!(mb));
                            }
                            "SwapTotal" => {
                                facts.insert("ansible_swaptotal_mb".to_string(), json!(mb));
                            }
                            "SwapFree" => {
                                facts.insert("ansible_swapfree_mb".to_string(), json!(mb));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(facts)
    }

    #[cfg(target_os = "macos")]
    async fn collect_macos_cpu_facts(
        &self,
    ) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();

        // Get CPU information using sysctl
        if let Ok(output) = tokio::process::Command::new("sysctl")
            .arg("-n")
            .arg("machdep.cpu.brand_string")
            .output()
            .await
        {
            if output.status.success() {
                let cpu_model = String::from_utf8_lossy(&output.stdout).trim().to_string();
                facts.insert("ansible_processor".to_string(), json!(vec![cpu_model]));
            }
        }

        // Get CPU counts
        if let Ok(output) = tokio::process::Command::new("sysctl")
            .arg("-n")
            .arg("hw.physicalcpu")
            .output()
            .await
        {
            if output.status.success() {
                if let Ok(count) = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .parse::<u32>()
                {
                    facts.insert("ansible_processor_cores".to_string(), json!(count));
                }
            }
        }

        if let Ok(output) = tokio::process::Command::new("sysctl")
            .arg("-n")
            .arg("hw.logicalcpu")
            .output()
            .await
        {
            if output.status.success() {
                if let Ok(count) = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .parse::<u32>()
                {
                    facts.insert("ansible_processor_vcpus".to_string(), json!(count));
                }
            }
        }

        facts.insert("ansible_processor_count".to_string(), json!(1));
        facts.insert("ansible_processor_threads_per_core".to_string(), json!(1));

        Ok(facts)
    }

    #[cfg(target_os = "macos")]
    async fn collect_macos_memory_facts(
        &self,
    ) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();

        // Get memory information using sysctl
        if let Ok(output) = tokio::process::Command::new("sysctl")
            .arg("-n")
            .arg("hw.memsize")
            .output()
            .await
        {
            if output.status.success() {
                if let Ok(bytes) = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .parse::<u64>()
                {
                    let mb = bytes / (1024 * 1024);
                    facts.insert("ansible_memtotal_mb".to_string(), json!(mb));
                }
            }
        }

        // Get free memory using vm_stat
        if let Ok(output) = tokio::process::Command::new("vm_stat").output().await {
            if output.status.success() {
                let vm_stat = String::from_utf8_lossy(&output.stdout);
                if let Some(free_pages) = self.parse_vm_stat(&vm_stat, "Pages free") {
                    let free_mb = (free_pages * 4096) / (1024 * 1024); // 4KB pages
                    facts.insert("ansible_memfree_mb".to_string(), json!(free_mb));
                }
            }
        }

        // macOS doesn't typically use swap in the traditional sense
        facts.insert("ansible_swaptotal_mb".to_string(), json!(0));
        facts.insert("ansible_swapfree_mb".to_string(), json!(0));

        Ok(facts)
    }

    #[cfg(target_os = "macos")]
    fn parse_vm_stat(&self, vm_stat: &str, key: &str) -> Option<u64> {
        for line in vm_stat.lines() {
            if line.starts_with(key) {
                if let Some(value_str) = line.split(':').nth(1) {
                    let value_str = value_str.trim().trim_end_matches('.');
                    if let Ok(value) = value_str.parse::<u64>() {
                        return Some(value);
                    }
                }
            }
        }
        None
    }

    #[cfg(target_os = "windows")]
    async fn collect_windows_cpu_facts(
        &self,
    ) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();

        // Use PowerShell to get CPU information
        if let Ok(output) = tokio::process::Command::new("powershell")
            .arg("-Command")
            .arg("Get-WmiObject -Class Win32_Processor | Select-Object Name, NumberOfCores, NumberOfLogicalProcessors | ConvertTo-Json")
            .output()
            .await
        {
            if output.status.success() {
                // Parse JSON output from PowerShell
                // This is a simplified implementation
                facts.insert("ansible_processor_count".to_string(), json!(1));
                facts.insert("ansible_processor_cores".to_string(), json!(num_cpus::get()));
                facts.insert("ansible_processor_vcpus".to_string(), json!(num_cpus::get()));
                facts.insert("ansible_processor_threads_per_core".to_string(), json!(1));
            }
        }

        Ok(facts)
    }

    #[cfg(target_os = "windows")]
    async fn collect_windows_memory_facts(
        &self,
    ) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();

        // Use PowerShell to get memory information
        if let Ok(output) = tokio::process::Command::new("powershell")
            .arg("-Command")
            .arg("Get-WmiObject -Class Win32_ComputerSystem | Select-Object TotalPhysicalMemory | ConvertTo-Json")
            .output()
            .await
        {
            if output.status.success() {
                // Parse JSON output from PowerShell
                // This is a simplified implementation
                facts.insert("ansible_memtotal_mb".to_string(), json!(8192)); // Default value
                facts.insert("ansible_memfree_mb".to_string(), json!(4096));
                facts.insert("ansible_swaptotal_mb".to_string(), json!(2048));
                facts.insert("ansible_swapfree_mb".to_string(), json!(1024));
            }
        }

        Ok(facts)
    }
}
