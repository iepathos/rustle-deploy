use crate::modules::error::ValidationError;
use crate::modules::loader::{LoadedModule, SecurityLevel};
use anyhow::Result;
use regex::Regex;
use tracing::{debug, warn};

/// Module validator for security and compatibility
pub struct ModuleValidator {
    security_policy: SecurityPolicy,
    compatibility_checker: CompatibilityChecker,
}

impl ModuleValidator {
    pub fn new() -> Self {
        Self {
            security_policy: SecurityPolicy::default(),
            compatibility_checker: CompatibilityChecker::new(),
        }
    }

    pub fn validate_module(
        &self,
        module: &LoadedModule,
    ) -> Result<ValidationResult, ValidationError> {
        let mut result = ValidationResult::new();

        // Security validation
        result.merge(self.validate_security(module)?);

        // Compatibility validation
        result.merge(self.validate_compatibility(module)?);

        // Dependency validation
        result.merge(self.validate_dependencies(module)?);

        // Code quality validation
        result.merge(self.validate_code_quality(module)?);

        Ok(result)
    }

    fn validate_security(
        &self,
        module: &LoadedModule,
    ) -> Result<ValidationResult, ValidationError> {
        let mut result = ValidationResult::new();

        // Check for dangerous operations
        let dangerous_patterns = [
            (r"std::process::Command", "process execution"),
            (r"std::fs::", "filesystem access"),
            (r"std::net::", "network access"),
            (r"unsafe\s*\{", "unsafe code block"),
            (r"#!\[.*unsafe.*\]", "unsafe attribute"),
            (r"std::env::set_var", "environment variable modification"),
            (r"std::mem::transmute", "memory transmutation"),
            (r"std::ptr::", "raw pointer manipulation"),
        ];

        for (pattern, description) in &dangerous_patterns {
            if self.contains_pattern(&module.source_code.main_file, pattern) {
                // For now, assume sandboxed security level for non-builtin modules
                let security_level = if matches!(
                    module.spec.source,
                    crate::execution::plan::ModuleSource::Builtin
                ) {
                    SecurityLevel::Trusted
                } else {
                    SecurityLevel::Sandboxed
                };
                match security_level {
                    SecurityLevel::Isolated => {
                        result.add_error(format!(
                            "Isolated module contains forbidden operation: {}",
                            description
                        ));
                    }
                    SecurityLevel::Sandboxed => {
                        // Check if this specific operation is allowed for sandboxed modules
                        if matches!(
                            *description,
                            "unsafe code block"
                                | "memory transmutation"
                                | "raw pointer manipulation"
                        ) {
                            result.add_error(format!(
                                "Sandboxed module contains forbidden operation: {}",
                                description
                            ));
                        } else {
                            result.add_warning(format!(
                                "Sandboxed module contains restricted operation: {}",
                                description
                            ));
                        }
                    }
                    SecurityLevel::Trusted => {
                        // Allowed for trusted modules, but still warn about unsafe code
                        if description.contains("unsafe") {
                            result.add_warning(format!(
                                "Trusted module contains potentially dangerous operation: {}",
                                description
                            ));
                        }
                    }
                }
            }
        }

        // Check for suspicious patterns
        let suspicious_patterns = [
            (r"include_bytes!", "embedded binary data"),
            (r"include_str!", "embedded string data"),
            (r"std::panic::set_hook", "panic handler modification"),
            (r"#\[link.*\]", "native library linking"),
        ];

        for (pattern, description) in &suspicious_patterns {
            if self.contains_pattern(&module.source_code.main_file, pattern) {
                result.add_warning(format!(
                    "Module contains suspicious pattern: {}",
                    description
                ));
            }
        }

        // Validate dependencies for security issues
        if let Some(cargo_toml) = &module.source_code.cargo_toml {
            self.validate_cargo_dependencies(cargo_toml, &mut result);
        }

        Ok(result)
    }

    fn validate_compatibility(
        &self,
        _module: &LoadedModule,
    ) -> Result<ValidationResult, ValidationError> {
        let result = ValidationResult::new();

        // Skip advanced compatibility checks for now since they're not in the simplified ModuleSpec
        // In production, this information could be extracted from the module manifest or cargo.toml

        Ok(result)
    }

    fn validate_dependencies(
        &self,
        module: &LoadedModule,
    ) -> Result<ValidationResult, ValidationError> {
        let mut result = ValidationResult::new();

        // Check for circular dependencies
        let mut visited = std::collections::HashSet::new();
        if self.has_circular_dependency(&module.spec.name, module, &mut visited) {
            result.add_error("Module has circular dependencies".to_string());
        }

        // Validate each dependency
        for dep in &module.resolved_dependencies {
            let dep_result = self.validate_module(dep)?;
            if !dep_result.passed {
                result.add_error(format!(
                    "Dependency '{}' validation failed: {:?}",
                    dep.spec.name, dep_result.errors
                ));
            }
        }

        Ok(result)
    }

    fn validate_code_quality(
        &self,
        module: &LoadedModule,
    ) -> Result<ValidationResult, ValidationError> {
        let mut result = ValidationResult::new();

        // Check for common code quality issues
        let quality_patterns = [
            (r"todo!?\(\)", "unimplemented code (todo! macro)"),
            (r"unimplemented!?\(\)", "unimplemented code"),
            (r"unreachable!?\(\)", "unreachable code"),
            (r"\.unwrap\(\)", "potential panic (unwrap)"),
            (r"\.expect\(", "potential panic (expect)"),
            (r"println!", "debug output"),
            (r"eprintln!", "debug error output"),
        ];

        for (pattern, description) in &quality_patterns {
            if self.contains_pattern(&module.source_code.main_file, pattern) {
                result.add_warning(format!("Code quality issue: {}", description));
            }
        }

        // Check module size
        let code_size = module.source_code.main_file.len()
            + module
                .source_code
                .additional_files
                .values()
                .map(|s| s.len())
                .sum::<usize>();

        if code_size > 1024 * 1024 {
            // 1MB
            result.add_warning(format!(
                "Module is very large ({} bytes), may impact compilation time",
                code_size
            ));
        }

        Ok(result)
    }

    fn contains_pattern(&self, code: &str, pattern: &str) -> bool {
        if let Ok(regex) = Regex::new(pattern) {
            regex.is_match(code)
        } else {
            warn!("Invalid regex pattern: {}", pattern);
            false
        }
    }

    fn validate_cargo_dependencies(&self, cargo_toml: &str, result: &mut ValidationResult) {
        // Simple TOML parsing for dependencies
        if cargo_toml.contains("[dependencies]") {
            // Check for known problematic crates
            let blocked_crates = &self.security_policy.blocked_crates;
            for crate_name in blocked_crates {
                if cargo_toml.contains(&format!("{} =", crate_name))
                    || cargo_toml.contains(&format!("{} {{", crate_name))
                {
                    result.add_error(format!("Module depends on blocked crate: {}", crate_name));
                }
            }

            // Warn about git dependencies
            if cargo_toml.contains("git = ") {
                result.add_warning(
                    "Module has git dependencies, which may pose security risks".to_string(),
                );
            }

            // Warn about path dependencies
            if cargo_toml.contains("path = ") {
                result.add_warning("Module has local path dependencies".to_string());
            }
        }
    }

    fn has_circular_dependency(
        &self,
        root_name: &str,
        module: &LoadedModule,
        visited: &mut std::collections::HashSet<String>,
    ) -> bool {
        if !visited.insert(module.spec.name.clone()) {
            return module.spec.name == root_name;
        }

        for dep in &module.resolved_dependencies {
            if self.has_circular_dependency(root_name, dep, visited) {
                return true;
            }
        }

        false
    }
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub passed: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub info: Vec<String>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            passed: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            info: Vec::new(),
        }
    }

    pub fn add_error(&mut self, error: String) {
        debug!("Validation error: {}", error);
        self.errors.push(error);
        self.passed = false;
    }

    pub fn add_warning(&mut self, warning: String) {
        debug!("Validation warning: {}", warning);
        self.warnings.push(warning);
    }

    pub fn add_info(&mut self, info: String) {
        debug!("Validation info: {}", info);
        self.info.push(info);
    }

    pub fn merge(&mut self, other: ValidationResult) {
        if !other.passed {
            self.passed = false;
        }
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
        self.info.extend(other.info);
    }
}

#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    pub allow_unsafe_code: bool,
    pub allow_network_access: bool,
    pub allow_filesystem_access: bool,
    pub allow_process_execution: bool,
    pub allowed_crates: Vec<String>,
    pub blocked_crates: Vec<String>,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            allow_unsafe_code: false,
            allow_network_access: true,
            allow_filesystem_access: true,
            allow_process_execution: true,
            allowed_crates: vec![
                "serde".to_string(),
                "serde_json".to_string(),
                "tokio".to_string(),
                "anyhow".to_string(),
                "thiserror".to_string(),
                "tracing".to_string(),
                "regex".to_string(),
                "chrono".to_string(),
            ],
            blocked_crates: vec![
                // Add known malicious or problematic crates here
            ],
        }
    }
}

pub struct CompatibilityChecker {
    current_rust_version: String,
    supported_platforms: Vec<String>,
    available_capabilities: Vec<String>,
}

impl CompatibilityChecker {
    pub fn new() -> Self {
        Self {
            current_rust_version: env!("CARGO_PKG_RUST_VERSION").to_string(),
            supported_platforms: vec![
                "x86_64-unknown-linux-gnu".to_string(),
                "x86_64-apple-darwin".to_string(),
                "aarch64-apple-darwin".to_string(),
                "x86_64-pc-windows-msvc".to_string(),
                "aarch64-unknown-linux-gnu".to_string(),
            ],
            available_capabilities: vec![
                "async".to_string(),
                "filesystem".to_string(),
                "network".to_string(),
                "process".to_string(),
                "systemd".to_string(),
                "launchd".to_string(),
                "winservice".to_string(),
            ],
        }
    }

    pub fn is_rust_version_compatible(&self, required: &str) -> Result<bool> {
        // Simple version comparison - in production, use semver crate
        Ok(self.current_rust_version >= required.to_string())
    }

    pub fn is_platform_supported(&self, platform: &str) -> bool {
        self.supported_platforms
            .iter()
            .any(|p| p == platform || p.contains(platform))
    }

    pub fn is_capability_available(&self, capability: &str) -> bool {
        self.available_capabilities.iter().any(|c| c == capability)
    }
}
