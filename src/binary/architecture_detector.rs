use anyhow::{anyhow, Result};
use std::collections::HashMap;

pub struct ArchitectureDetector {
    default_architecture: String,
    architecture_cache: HashMap<String, String>,
}

impl ArchitectureDetector {
    pub fn new() -> Self {
        Self {
            default_architecture: "x86_64-unknown-linux-gnu".to_string(),
            architecture_cache: HashMap::new(),
        }
    }

    pub fn detect_primary_architecture(&self, hosts: &[String]) -> Result<String> {
        if hosts.is_empty() {
            return Err(anyhow!("No hosts provided for architecture detection"));
        }

        // For now, return default architecture
        // In a real implementation, this would:
        // 1. SSH into each host
        // 2. Run `uname -m` and `uname -s`
        // 3. Map the results to Rust target triples
        // 4. Return the most common architecture

        Ok(self.default_architecture.clone())
    }

    pub fn detect_host_architecture(&mut self, host: &str) -> Result<String> {
        // Check cache first
        if let Some(cached_arch) = self.architecture_cache.get(host) {
            return Ok(cached_arch.clone());
        }

        // In a real implementation, this would SSH to the host and detect architecture
        let architecture = self.detect_architecture_for_host(host)?;

        // Cache the result
        self.architecture_cache
            .insert(host.to_string(), architecture.clone());

        Ok(architecture)
    }

    fn detect_architecture_for_host(&self, host: &str) -> Result<String> {
        // Mock implementation - in reality this would SSH and run commands
        match host {
            "localhost" => {
                // Try to detect local architecture
                self.detect_local_architecture()
            }
            _ => {
                // For remote hosts, use default for now
                Ok(self.default_architecture.clone())
            }
        }
    }

    fn detect_local_architecture(&self) -> Result<String> {
        // Try to detect the local system architecture
        #[cfg(target_arch = "x86_64")]
        let arch = "x86_64";
        #[cfg(target_arch = "aarch64")]
        let arch = "aarch64";
        #[cfg(target_arch = "arm")]
        let arch = "arm";
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "arm")))]
        let arch = "unknown";

        #[cfg(target_os = "linux")]
        let os = "unknown-linux-gnu";
        #[cfg(target_os = "macos")]
        let os = "apple-darwin";
        #[cfg(target_os = "windows")]
        let os = "pc-windows-msvc";
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        let os = "unknown";

        Ok(format!("{arch}-{os}"))
    }

    pub fn validate_target_triple(&self, target_triple: &str) -> Result<bool> {
        // Basic validation for target triple format
        let parts: Vec<&str> = target_triple.split('-').collect();

        if parts.len() < 3 {
            return Ok(false);
        }

        let arch = parts[0];
        let vendor = parts[1];
        let os_parts = &parts[2..];

        // Validate architecture
        let valid_archs = [
            "x86_64",
            "i686",
            "aarch64",
            "arm",
            "armv7",
            "armv6",
            "mips",
            "mips64",
            "powerpc",
            "powerpc64",
            "riscv64",
            "s390x",
        ];
        if !valid_archs.contains(&arch) {
            return Ok(false);
        }

        // Validate vendor
        let valid_vendors = ["unknown", "pc", "apple", "linux"];
        if !valid_vendors.contains(&vendor) {
            return Ok(false);
        }

        // Basic OS validation
        let os_string = os_parts.join("-");
        let valid_os_patterns = [
            "linux-gnu",
            "linux-musl",
            "darwin",
            "windows-msvc",
            "windows-gnu",
            "freebsd",
            "netbsd",
            "openbsd",
        ];

        let is_valid_os = valid_os_patterns
            .iter()
            .any(|pattern| os_string.contains(pattern));

        Ok(is_valid_os)
    }

    pub fn normalize_target_triple(&self, target_triple: &str) -> Result<String> {
        // Common normalizations first
        let normalized = match target_triple {
            "x86_64-linux-gnu" => "x86_64-unknown-linux-gnu",
            "arm64-apple-darwin" => "aarch64-apple-darwin",
            "x86_64-apple-darwin" => "x86_64-apple-darwin",
            "x86_64-windows" => "x86_64-pc-windows-msvc",
            _ => target_triple,
        };

        // Validate the normalized form
        if !self.validate_target_triple(normalized)? {
            return Err(anyhow!("Invalid target triple: {}", target_triple));
        }

        Ok(normalized.to_string())
    }

    pub fn get_cross_compilation_requirements(&self, target: &str) -> Result<CrossCompilationInfo> {
        let normalized_target = self.normalize_target_triple(target)?;
        let parts: Vec<&str> = normalized_target.split('-').collect();

        if parts.len() < 3 {
            return Err(anyhow!("Invalid target triple format"));
        }

        let arch = parts[0];
        let _vendor = parts[1];
        let os_env = parts[2..].join("-");

        let cross_compiler_required = self.requires_cross_compilation(arch, &os_env);
        let linker_requirements = self.get_linker_requirements(arch, &os_env);
        let system_dependencies = self.get_system_dependencies(arch, &os_env);
        let docker_image = self.get_docker_image(arch, &os_env);

        Ok(CrossCompilationInfo {
            target_triple: normalized_target,
            requires_cross_compiler: cross_compiler_required,
            linker_requirements,
            system_dependencies,
            docker_image,
        })
    }

    fn requires_cross_compilation(&self, target_arch: &str, target_os: &str) -> bool {
        let local_arch = self
            .detect_local_architecture()
            .unwrap_or_else(|_| self.default_architecture.clone());

        // If target architecture or OS differs from local, cross-compilation is needed
        !local_arch.starts_with(target_arch) || !local_arch.contains(target_os)
    }

    fn get_linker_requirements(&self, arch: &str, os_env: &str) -> Vec<String> {
        let mut requirements = Vec::new();

        match (arch, os_env) {
            ("aarch64", os) if os.contains("linux") => {
                requirements.push("gcc-aarch64-linux-gnu".to_string());
            }
            ("arm", os) if os.contains("linux") => {
                requirements.push("gcc-arm-linux-gnueabihf".to_string());
            }
            ("x86_64", os) if os.contains("windows") => {
                requirements.push("mingw-w64".to_string());
            }
            _ => {}
        }

        requirements
    }

    fn get_system_dependencies(&self, arch: &str, os_env: &str) -> Vec<String> {
        let mut deps = Vec::new();

        if os_env.contains("linux") && arch != "x86_64" {
            deps.push(format!("libc6-dev-{arch}-cross"));
        }

        if os_env.contains("musl") {
            deps.push("musl-tools".to_string());
        }

        deps
    }

    fn get_docker_image(&self, arch: &str, os_env: &str) -> Option<String> {
        match (arch, os_env) {
            ("x86_64", os) if os.contains("linux-gnu") => Some("rust:latest".to_string()),
            ("aarch64", os) if os.contains("linux-gnu") => {
                Some("rustembedded/cross:aarch64-unknown-linux-gnu".to_string())
            }
            ("arm", os) if os.contains("linux-gnueabihf") => {
                Some("rustembedded/cross:arm-unknown-linux-gnueabihf".to_string())
            }
            _ => None,
        }
    }

    pub fn clear_cache(&mut self) {
        self.architecture_cache.clear();
    }

    pub fn set_default_architecture(&mut self, arch: String) -> Result<()> {
        if !self.validate_target_triple(&arch)? {
            return Err(anyhow!("Invalid default architecture: {}", arch));
        }
        self.default_architecture = self.normalize_target_triple(&arch)?;
        Ok(())
    }
}

impl Default for ArchitectureDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct CrossCompilationInfo {
    pub target_triple: String,
    pub requires_cross_compiler: bool,
    pub linker_requirements: Vec<String>,
    pub system_dependencies: Vec<String>,
    pub docker_image: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_creation() {
        let detector = ArchitectureDetector::new();
        assert_eq!(detector.default_architecture, "x86_64-unknown-linux-gnu");
    }

    #[test]
    fn test_detect_primary_architecture() {
        let detector = ArchitectureDetector::new();
        let hosts = vec!["localhost".to_string()];

        let result = detector.detect_primary_architecture(&hosts);
        assert!(result.is_ok());
    }

    #[test]
    fn test_detect_primary_architecture_empty_hosts() {
        let detector = ArchitectureDetector::new();
        let hosts = vec![];

        let result = detector.detect_primary_architecture(&hosts);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_target_triple() {
        let detector = ArchitectureDetector::new();

        assert!(detector
            .validate_target_triple("x86_64-unknown-linux-gnu")
            .unwrap());
        assert!(detector
            .validate_target_triple("aarch64-apple-darwin")
            .unwrap());
        assert!(!detector.validate_target_triple("invalid").unwrap());
        assert!(!detector.validate_target_triple("x86_64-").unwrap());
    }

    #[test]
    fn test_normalize_target_triple() {
        let detector = ArchitectureDetector::new();

        let result = detector.normalize_target_triple("x86_64-linux-gnu");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "x86_64-unknown-linux-gnu");

        let result = detector.normalize_target_triple("arm64-apple-darwin");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "aarch64-apple-darwin");

        let result = detector.normalize_target_triple("x86_64-unknown-linux-gnu");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "x86_64-unknown-linux-gnu");
    }

    #[test]
    fn test_get_cross_compilation_requirements() {
        let detector = ArchitectureDetector::new();

        let result = detector.get_cross_compilation_requirements("x86_64-unknown-linux-gnu");
        assert!(result.is_ok());

        let info = result.unwrap();
        assert_eq!(info.target_triple, "x86_64-unknown-linux-gnu");
    }

    #[test]
    fn test_detect_host_architecture() {
        let mut detector = ArchitectureDetector::new();

        let result = detector.detect_host_architecture("localhost");
        assert!(result.is_ok());

        // Test caching
        let result2 = detector.detect_host_architecture("localhost");
        assert!(result2.is_ok());
        assert_eq!(result.unwrap(), result2.unwrap());
    }

    #[test]
    fn test_clear_cache() {
        let mut detector = ArchitectureDetector::new();

        // Populate cache
        let _ = detector.detect_host_architecture("localhost");
        assert!(!detector.architecture_cache.is_empty());

        // Clear cache
        detector.clear_cache();
        assert!(detector.architecture_cache.is_empty());
    }

    #[test]
    fn test_set_default_architecture() {
        let mut detector = ArchitectureDetector::new();

        let result = detector.set_default_architecture("aarch64-apple-darwin".to_string());
        assert!(result.is_ok());
        assert_eq!(detector.default_architecture, "aarch64-apple-darwin");

        let result = detector.set_default_architecture("invalid".to_string());
        assert!(result.is_err());
    }
}
