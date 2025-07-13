use anyhow::{anyhow, Result};
use std::collections::HashMap;

use crate::execution::compatibility::AssessmentError;
use crate::execution::rustle_plan::BinaryCompatibility;

pub struct ModuleRegistry {
    compatibility_db: HashMap<String, ModuleCompatibilityInfo>,
    custom_modules: HashMap<String, CustomModuleInfo>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            compatibility_db: HashMap::new(),
            custom_modules: HashMap::new(),
        };

        registry.populate_builtin_modules();
        registry
    }

    fn populate_builtin_modules(&mut self) {
        // Core modules - fully compatible
        self.register_module(
            "debug",
            ModuleCompatibilityInfo {
                compatibility: BinaryCompatibility::FullyCompatible,
                static_linkable: true,
                performance_impact: PerformanceImpact::Low,
                resource_requirements: ResourceRequirements::minimal(),
                dependencies: vec![],
                version_constraints: None,
            },
        );

        self.register_module(
            "set_fact",
            ModuleCompatibilityInfo {
                compatibility: BinaryCompatibility::FullyCompatible,
                static_linkable: true,
                performance_impact: PerformanceImpact::Low,
                resource_requirements: ResourceRequirements::minimal(),
                dependencies: vec![],
                version_constraints: None,
            },
        );

        self.register_module(
            "assert",
            ModuleCompatibilityInfo {
                compatibility: BinaryCompatibility::FullyCompatible,
                static_linkable: true,
                performance_impact: PerformanceImpact::Low,
                resource_requirements: ResourceRequirements::minimal(),
                dependencies: vec![],
                version_constraints: None,
            },
        );

        // File operations - mostly compatible
        self.register_module(
            "copy",
            ModuleCompatibilityInfo {
                compatibility: BinaryCompatibility::PartiallyCompatible {
                    limitations: vec![
                        "Remote source files require SSH access".to_string(),
                        "Backup operations may not be fully supported".to_string(),
                    ],
                },
                static_linkable: true,
                performance_impact: PerformanceImpact::Medium,
                resource_requirements: ResourceRequirements::moderate(),
                dependencies: vec!["filesystem".to_string()],
                version_constraints: None,
            },
        );

        self.register_module(
            "template",
            ModuleCompatibilityInfo {
                compatibility: BinaryCompatibility::PartiallyCompatible {
                    limitations: vec![
                        "Complex Jinja2 features may be limited".to_string(),
                        "External template includes not supported".to_string(),
                    ],
                },
                static_linkable: true,
                performance_impact: PerformanceImpact::Medium,
                resource_requirements: ResourceRequirements::moderate(),
                dependencies: vec!["templating".to_string()],
                version_constraints: None,
            },
        );

        self.register_module(
            "file",
            ModuleCompatibilityInfo {
                compatibility: BinaryCompatibility::PartiallyCompatible {
                    limitations: vec!["Complex file attribute operations".to_string()],
                },
                static_linkable: true,
                performance_impact: PerformanceImpact::Medium,
                resource_requirements: ResourceRequirements::moderate(),
                dependencies: vec!["filesystem".to_string()],
                version_constraints: None,
            },
        );

        // Command execution - limited compatibility
        self.register_module(
            "command",
            ModuleCompatibilityInfo {
                compatibility: BinaryCompatibility::PartiallyCompatible {
                    limitations: vec![
                        "Command output may vary between environments".to_string(),
                        "Interactive commands not supported".to_string(),
                        "Path dependencies may not be available".to_string(),
                    ],
                },
                static_linkable: false,
                performance_impact: PerformanceImpact::High,
                resource_requirements: ResourceRequirements::high(),
                dependencies: vec!["process".to_string()],
                version_constraints: None,
            },
        );

        self.register_module(
            "shell",
            ModuleCompatibilityInfo {
                compatibility: BinaryCompatibility::PartiallyCompatible {
                    limitations: vec![
                        "Shell environment differences".to_string(),
                        "Shell-specific features may not work".to_string(),
                        "Environment variables may differ".to_string(),
                    ],
                },
                static_linkable: false,
                performance_impact: PerformanceImpact::High,
                resource_requirements: ResourceRequirements::high(),
                dependencies: vec!["process", "shell"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                version_constraints: None,
            },
        );

        // System management - mostly incompatible
        self.register_module(
            "package",
            ModuleCompatibilityInfo {
                compatibility: BinaryCompatibility::Incompatible {
                    reasons: vec![
                        "Requires system package manager access".to_string(),
                        "Package state management needs root privileges".to_string(),
                        "Distribution-specific package formats".to_string(),
                    ],
                },
                static_linkable: false,
                performance_impact: PerformanceImpact::High,
                resource_requirements: ResourceRequirements::high(),
                dependencies: vec!["system", "package-manager"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                version_constraints: None,
            },
        );

        self.register_module(
            "service",
            ModuleCompatibilityInfo {
                compatibility: BinaryCompatibility::Incompatible {
                    reasons: vec![
                        "Requires systemd or init system access".to_string(),
                        "Service management needs root privileges".to_string(),
                        "Platform-specific service formats".to_string(),
                    ],
                },
                static_linkable: false,
                performance_impact: PerformanceImpact::High,
                resource_requirements: ResourceRequirements::high(),
                dependencies: vec!["system", "service-manager"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                version_constraints: None,
            },
        );

        self.register_module(
            "user",
            ModuleCompatibilityInfo {
                compatibility: BinaryCompatibility::Incompatible {
                    reasons: vec![
                        "User management requires root access".to_string(),
                        "System user database modifications".to_string(),
                        "Platform-specific user management tools".to_string(),
                    ],
                },
                static_linkable: false,
                performance_impact: PerformanceImpact::High,
                resource_requirements: ResourceRequirements::high(),
                dependencies: vec!["system", "user-management"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                version_constraints: None,
            },
        );

        self.register_module(
            "mount",
            ModuleCompatibilityInfo {
                compatibility: BinaryCompatibility::Incompatible {
                    reasons: vec![
                        "Filesystem mounting requires root privileges".to_string(),
                        "Kernel-level filesystem operations".to_string(),
                        "Platform-specific mount utilities".to_string(),
                    ],
                },
                static_linkable: false,
                performance_impact: PerformanceImpact::High,
                resource_requirements: ResourceRequirements::high(),
                dependencies: vec!["system", "filesystem"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                version_constraints: None,
            },
        );

        // Network modules - partially compatible
        self.register_module(
            "uri",
            ModuleCompatibilityInfo {
                compatibility: BinaryCompatibility::PartiallyCompatible {
                    limitations: vec![
                        "SSL certificate validation may differ".to_string(),
                        "Proxy settings may not be inherited".to_string(),
                    ],
                },
                static_linkable: true,
                performance_impact: PerformanceImpact::Medium,
                resource_requirements: ResourceRequirements::moderate(),
                dependencies: vec!["http", "tls"].into_iter().map(String::from).collect(),
                version_constraints: None,
            },
        );

        self.register_module(
            "get_url",
            ModuleCompatibilityInfo {
                compatibility: BinaryCompatibility::PartiallyCompatible {
                    limitations: vec![
                        "Authentication methods may be limited".to_string(),
                        "Resume functionality may not work".to_string(),
                    ],
                },
                static_linkable: true,
                performance_impact: PerformanceImpact::Medium,
                resource_requirements: ResourceRequirements::moderate(),
                dependencies: vec!["http", "filesystem"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                version_constraints: None,
            },
        );

        // Interactive modules - incompatible
        self.register_module(
            "pause",
            ModuleCompatibilityInfo {
                compatibility: BinaryCompatibility::Incompatible {
                    reasons: vec![
                        "Requires interactive user input".to_string(),
                        "Cannot be automated in binary deployment".to_string(),
                    ],
                },
                static_linkable: false,
                performance_impact: PerformanceImpact::Low,
                resource_requirements: ResourceRequirements::minimal(),
                dependencies: vec![],
                version_constraints: None,
            },
        );

        self.register_module(
            "wait_for",
            ModuleCompatibilityInfo {
                compatibility: BinaryCompatibility::FullyCompatible,
                static_linkable: true,
                performance_impact: PerformanceImpact::Low,
                resource_requirements: ResourceRequirements::minimal(),
                dependencies: vec!["network".to_string()],
                version_constraints: None,
            },
        );
    }

    pub fn register_module(&mut self, name: &str, info: ModuleCompatibilityInfo) {
        self.compatibility_db.insert(name.to_string(), info);
    }

    pub fn register_custom_module(&mut self, name: &str, info: CustomModuleInfo) {
        self.custom_modules.insert(name.to_string(), info);
    }

    pub fn check_module_compatibility(
        &self,
        module_name: &str,
    ) -> Result<BinaryCompatibility, AssessmentError> {
        // Check built-in modules first
        if let Some(info) = self.compatibility_db.get(module_name) {
            return Ok(info.compatibility.clone());
        }

        // Check custom modules
        if let Some(custom_info) = self.custom_modules.get(module_name) {
            return Ok(custom_info.estimated_compatibility.clone());
        }

        // Unknown module - conservative assessment
        Ok(BinaryCompatibility::PartiallyCompatible {
            limitations: vec![
                format!(
                    "Unknown module '{}' - compatibility not verified",
                    module_name
                ),
                "May require additional dependencies".to_string(),
            ],
        })
    }

    pub fn get_module_info(&self, module_name: &str) -> Option<&ModuleCompatibilityInfo> {
        self.compatibility_db.get(module_name)
    }

    pub fn get_custom_module_info(&self, module_name: &str) -> Option<&CustomModuleInfo> {
        self.custom_modules.get(module_name)
    }

    pub fn list_compatible_modules(&self) -> Vec<String> {
        self.compatibility_db
            .iter()
            .filter(|(_, info)| {
                matches!(
                    info.compatibility,
                    BinaryCompatibility::FullyCompatible
                        | BinaryCompatibility::PartiallyCompatible { .. }
                )
            })
            .map(|(name, _)| name.clone())
            .collect()
    }

    pub fn list_incompatible_modules(&self) -> Vec<String> {
        self.compatibility_db
            .iter()
            .filter(|(_, info)| {
                matches!(info.compatibility, BinaryCompatibility::Incompatible { .. })
            })
            .map(|(name, _)| name.clone())
            .collect()
    }

    pub fn get_dependencies(&self, module_name: &str) -> Vec<String> {
        self.compatibility_db
            .get(module_name)
            .map(|info| info.dependencies.clone())
            .unwrap_or_default()
    }

    pub fn is_static_linkable(&self, module_name: &str) -> bool {
        self.compatibility_db
            .get(module_name)
            .map(|info| info.static_linkable)
            .unwrap_or(false)
    }

    pub fn get_performance_impact(&self, module_name: &str) -> PerformanceImpact {
        self.compatibility_db
            .get(module_name)
            .map(|info| info.performance_impact.clone())
            .unwrap_or(PerformanceImpact::Medium)
    }

    pub fn update_module_compatibility(
        &mut self,
        module_name: &str,
        compatibility: BinaryCompatibility,
    ) -> Result<()> {
        if let Some(info) = self.compatibility_db.get_mut(module_name) {
            info.compatibility = compatibility;
            Ok(())
        } else {
            Err(anyhow!("Module '{}' not found in registry", module_name))
        }
    }

    pub fn analyze_module_set(&self, modules: &[String]) -> ModuleSetAnalysis {
        let mut analysis = ModuleSetAnalysis {
            total_modules: modules.len(),
            fully_compatible: 0,
            partially_compatible: 0,
            incompatible: 0,
            unknown: 0,
            static_linkable: 0,
            total_dependencies: Vec::new(),
            performance_score: 0.0,
        };

        let mut total_performance = 0.0;

        for module in modules {
            match self.check_module_compatibility(module) {
                Ok(BinaryCompatibility::FullyCompatible) => {
                    analysis.fully_compatible += 1;
                    total_performance += 1.0;
                }
                Ok(BinaryCompatibility::PartiallyCompatible { .. }) => {
                    analysis.partially_compatible += 1;
                    total_performance += 0.6;
                }
                Ok(BinaryCompatibility::Incompatible { .. }) => {
                    analysis.incompatible += 1;
                    total_performance += 0.0;
                }
                Err(_) => {
                    analysis.unknown += 1;
                    total_performance += 0.3;
                }
            }

            if self.is_static_linkable(module) {
                analysis.static_linkable += 1;
            }

            analysis
                .total_dependencies
                .extend(self.get_dependencies(module));
        }

        // Remove duplicate dependencies
        analysis.total_dependencies.sort();
        analysis.total_dependencies.dedup();

        // Calculate performance score
        analysis.performance_score = if modules.is_empty() {
            0.0
        } else {
            total_performance / modules.len() as f32
        };

        analysis
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ModuleCompatibilityInfo {
    pub compatibility: BinaryCompatibility,
    pub static_linkable: bool,
    pub performance_impact: PerformanceImpact,
    pub resource_requirements: ResourceRequirements,
    pub dependencies: Vec<String>,
    pub version_constraints: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CustomModuleInfo {
    pub source_path: String,
    pub estimated_compatibility: BinaryCompatibility,
    pub build_requirements: Vec<String>,
    pub runtime_dependencies: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum PerformanceImpact {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone)]
pub struct ResourceRequirements {
    pub memory_mb: u32,
    pub cpu_cores: f32,
    pub disk_mb: u32,
    pub network_required: bool,
}

impl ResourceRequirements {
    pub fn minimal() -> Self {
        Self {
            memory_mb: 16,
            cpu_cores: 0.1,
            disk_mb: 1,
            network_required: false,
        }
    }

    pub fn moderate() -> Self {
        Self {
            memory_mb: 64,
            cpu_cores: 0.5,
            disk_mb: 10,
            network_required: false,
        }
    }

    pub fn high() -> Self {
        Self {
            memory_mb: 256,
            cpu_cores: 1.0,
            disk_mb: 100,
            network_required: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModuleSetAnalysis {
    pub total_modules: usize,
    pub fully_compatible: usize,
    pub partially_compatible: usize,
    pub incompatible: usize,
    pub unknown: usize,
    pub static_linkable: usize,
    pub total_dependencies: Vec<String>,
    pub performance_score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = ModuleRegistry::new();
        assert!(!registry.compatibility_db.is_empty());
    }

    #[test]
    fn test_check_builtin_module_compatibility() {
        let registry = ModuleRegistry::new();

        let result = registry.check_module_compatibility("debug");
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            BinaryCompatibility::FullyCompatible
        ));

        let result = registry.check_module_compatibility("package");
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            BinaryCompatibility::Incompatible { .. }
        ));
    }

    #[test]
    fn test_check_unknown_module_compatibility() {
        let registry = ModuleRegistry::new();

        let result = registry.check_module_compatibility("unknown_module");
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            BinaryCompatibility::PartiallyCompatible { .. }
        ));
    }

    #[test]
    fn test_list_compatible_modules() {
        let registry = ModuleRegistry::new();

        let compatible = registry.list_compatible_modules();
        assert!(!compatible.is_empty());
        assert!(compatible.contains(&"debug".to_string()));
    }

    #[test]
    fn test_list_incompatible_modules() {
        let registry = ModuleRegistry::new();

        let incompatible = registry.list_incompatible_modules();
        assert!(!incompatible.is_empty());
        assert!(incompatible.contains(&"package".to_string()));
    }

    #[test]
    fn test_is_static_linkable() {
        let registry = ModuleRegistry::new();

        assert!(registry.is_static_linkable("debug"));
        assert!(!registry.is_static_linkable("package"));
        assert!(!registry.is_static_linkable("unknown_module"));
    }

    #[test]
    fn test_get_dependencies() {
        let registry = ModuleRegistry::new();

        let deps = registry.get_dependencies("copy");
        assert!(!deps.is_empty());
        assert!(deps.contains(&"filesystem".to_string()));

        let deps = registry.get_dependencies("unknown_module");
        assert!(deps.is_empty());
    }

    #[test]
    fn test_register_custom_module() {
        let mut registry = ModuleRegistry::new();

        let custom_info = CustomModuleInfo {
            source_path: "/path/to/custom_module.py".to_string(),
            estimated_compatibility: BinaryCompatibility::PartiallyCompatible {
                limitations: vec!["Custom module limitations".to_string()],
            },
            build_requirements: vec!["python3".to_string()],
            runtime_dependencies: vec!["python3-dev".to_string()],
        };

        registry.register_custom_module("custom_module", custom_info);

        let result = registry.check_module_compatibility("custom_module");
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            BinaryCompatibility::PartiallyCompatible { .. }
        ));
    }

    #[test]
    fn test_analyze_module_set() {
        let registry = ModuleRegistry::new();
        let modules = vec![
            "debug".to_string(),
            "copy".to_string(),
            "package".to_string(),
        ];

        let analysis = registry.analyze_module_set(&modules);
        assert_eq!(analysis.total_modules, 3);
        assert!(analysis.fully_compatible > 0);
        assert!(analysis.partially_compatible > 0);
        assert!(analysis.incompatible > 0);
        assert!(analysis.performance_score > 0.0);
        assert!(analysis.performance_score <= 1.0);
    }

    #[test]
    fn test_update_module_compatibility() {
        let mut registry = ModuleRegistry::new();

        let result = registry.update_module_compatibility(
            "debug",
            BinaryCompatibility::PartiallyCompatible {
                limitations: vec!["Test limitation".to_string()],
            },
        );
        assert!(result.is_ok());

        let result = registry
            .update_module_compatibility("nonexistent", BinaryCompatibility::FullyCompatible);
        assert!(result.is_err());
    }
}
