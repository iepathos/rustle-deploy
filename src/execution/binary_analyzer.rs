use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

use super::compatibility::{AnalysisError, AssessmentError, CalculationError, EstimationError};
use super::rustle_plan::{
    BinaryCompatibility, BinaryDeploymentPlan, CompilationRequirements, TaskPlan,
};

pub struct BinaryDeploymentAnalyzer {
    module_registry: ModuleRegistry,
    architecture_detector: ArchitectureDetector,
}

impl BinaryDeploymentAnalyzer {
    pub fn new() -> Self {
        Self {
            module_registry: ModuleRegistry::new(),
            architecture_detector: ArchitectureDetector::new(),
        }
    }

    fn parse_arch_from_triple(triple: &str) -> String {
        if let Some(arch) = triple.split('-').next() {
            arch.to_string()
        } else {
            "x86_64".to_string()
        }
    }

    fn parse_os_from_triple(triple: &str) -> String {
        let parts: Vec<&str> = triple.split('-').collect();
        if parts.len() >= 3 {
            parts[2].to_string()
        } else {
            "linux".to_string()
        }
    }

    pub fn analyze_tasks_for_binary_deployment(
        &self,
        tasks: &[TaskPlan],
        hosts: &[String],
        threshold: u32,
    ) -> Result<Vec<BinaryDeploymentPlan>, AnalysisError> {
        let mut deployments = Vec::new();

        // Group tasks by compatibility and architecture
        let compatibility_groups = self.group_tasks_by_compatibility(tasks)?;

        for (architecture, compatible_tasks) in compatibility_groups {
            // Only create binary deployment if we have enough tasks
            if compatible_tasks.len() >= threshold as usize {
                let deployment_id = format!("binary-{}", Uuid::new_v4());
                let task_ids: Vec<String> =
                    compatible_tasks.iter().map(|t| t.task_id.clone()).collect();
                let modules = self.extract_required_modules(&compatible_tasks);
                let static_files = self.extract_static_files(&compatible_tasks);
                let estimated_savings = self.calculate_time_savings(&compatible_tasks)?;

                let deployment = BinaryDeploymentPlan {
                    deployment_id: deployment_id.clone(),
                    target_hosts: hosts.to_vec(),
                    binary_name: format!("rustle-runner-{deployment_id}"),
                    tasks: task_ids.clone(),
                    modules: modules.clone(),
                    embedded_data: Default::default(),
                    execution_mode: Default::default(),
                    estimated_size: 0,
                    compilation_requirements: CompilationRequirements {
                        target_arch: Self::parse_arch_from_triple(&architecture),
                        target_os: Self::parse_os_from_triple(&architecture),
                        rust_version: "1.70.0".to_string(),
                        cross_compilation: false,
                        static_linking: true,
                        modules: Some(modules),
                        static_files: Some(static_files),
                        target_triple: Some(architecture.clone()),
                        optimization_level: Some("release".to_string()),
                        features: Some(vec!["binary-deployment".to_string()]),
                    },
                    target_architecture: Some(architecture.clone()),
                    task_ids: Some(task_ids),
                    estimated_savings: Some(estimated_savings),
                    controller_endpoint: None,
                    execution_timeout: None,
                    report_interval: None,
                    cleanup_on_completion: None,
                    log_level: None,
                    max_retries: None,
                    static_files: vec![],
                    secrets: vec![],
                    verbose: None,
                };

                deployments.push(deployment);
            }
        }

        Ok(deployments)
    }

    pub fn estimate_compilation_time(
        &self,
        deployment: &BinaryDeploymentPlan,
    ) -> Result<Duration, EstimationError> {
        let base_time = Duration::from_secs(30); // Base compilation time
        let module_factor = deployment
            .compilation_requirements
            .modules
            .as_ref()
            .map(|m| m.len())
            .unwrap_or(0) as u64
            * 5;
        let feature_factor = deployment
            .compilation_requirements
            .features
            .as_ref()
            .map(|f| f.len())
            .unwrap_or(0) as u64
            * 2;

        // Add complexity based on optimization level
        let optimization_factor = match deployment
            .compilation_requirements
            .optimization_level
            .as_deref()
        {
            Some("debug") => 1,
            Some("release") => 3,
            Some("lto") => 5,
            _ => 2,
        };

        let total_seconds =
            base_time.as_secs() + module_factor + feature_factor + optimization_factor;
        Ok(Duration::from_secs(total_seconds))
    }

    pub fn calculate_network_savings(
        &self,
        tasks: &[TaskPlan],
        deployment_method: &str,
    ) -> Result<f32, CalculationError> {
        let total_tasks = tasks.len() as f32;

        if total_tasks == 0.0 {
            return Ok(0.0);
        }

        // Estimate network savings based on deployment method
        let savings_ratio = match deployment_method {
            "binary" => {
                // Binary deployment reduces network overhead for task execution
                let binary_compatible_count = tasks
                    .iter()
                    .filter(|task| self.assess_binary_compatibility(task).is_ok())
                    .count() as f32;

                (binary_compatible_count / total_tasks) * 0.8 // 80% savings for binary-compatible tasks
            }
            "hybrid" => {
                // Hybrid approach provides moderate savings
                0.4
            }
            "ssh" => {
                // Pure SSH has no network savings
                0.0
            }
            _ => 0.2, // Default moderate savings
        };

        Ok(savings_ratio.clamp(0.0, 1.0))
    }

    pub fn assess_binary_compatibility(
        &self,
        task: &TaskPlan,
    ) -> Result<BinaryCompatibility, AssessmentError> {
        let module_compatibility = self.module_registry.check_compatibility(&task.module)?;

        match module_compatibility {
            ModuleCompatibility::FullyCompatible => {
                // Check for incompatible arguments or conditions
                if self.has_interactive_requirements(task) {
                    Ok(BinaryCompatibility::PartiallyCompatible {
                        limitations: vec!["Interactive input required".to_string()],
                    })
                } else if self.has_dynamic_arguments(task) {
                    Ok(BinaryCompatibility::PartiallyCompatible {
                        limitations: vec!["Dynamic argument resolution required".to_string()],
                    })
                } else {
                    Ok(BinaryCompatibility::FullyCompatible)
                }
            }
            ModuleCompatibility::PartiallyCompatible { limitations } => {
                Ok(BinaryCompatibility::PartiallyCompatible { limitations })
            }
            ModuleCompatibility::Incompatible { reasons } => {
                Ok(BinaryCompatibility::Incompatible { reasons })
            }
        }
    }

    fn group_tasks_by_compatibility(
        &self,
        tasks: &[TaskPlan],
    ) -> Result<HashMap<String, Vec<TaskPlan>>, AnalysisError> {
        let mut groups = HashMap::new();

        for task in tasks {
            let compatibility = self.assess_binary_compatibility(task).map_err(|e| {
                AnalysisError::CompatibilityAnalysis {
                    task_id: task.task_id.clone(),
                    reason: e.to_string(),
                }
            })?;

            match compatibility {
                BinaryCompatibility::FullyCompatible => {
                    let arch = self.detect_target_architecture(&task.hosts)?;
                    groups
                        .entry(arch)
                        .or_insert_with(Vec::new)
                        .push(task.clone());
                }
                BinaryCompatibility::PartiallyCompatible { limitations } => {
                    // Log limitations but still include if major functionality works
                    if !limitations.iter().any(|l| l.contains("critical")) {
                        let arch = self.detect_target_architecture(&task.hosts)?;
                        groups
                            .entry(arch)
                            .or_insert_with(Vec::new)
                            .push(task.clone());
                    }
                }
                BinaryCompatibility::Incompatible { .. } => {
                    // Skip incompatible tasks
                    continue;
                }
            }
        }

        Ok(groups)
    }

    fn detect_target_architecture(&self, hosts: &[String]) -> Result<String, AnalysisError> {
        self.architecture_detector
            .detect_architecture(hosts)
            .map_err(|_e| AnalysisError::ArchitectureDetection {
                hosts: hosts.to_vec(),
            })
    }

    fn calculate_time_savings(&self, tasks: &[TaskPlan]) -> Result<Duration, EstimationError> {
        let total_estimated = tasks
            .iter()
            .map(|task| task.estimated_duration)
            .sum::<Duration>();

        // Binary deployment typically saves 20-40% of execution time
        let savings_ratio = 0.3;
        let savings_nanos = (total_estimated.as_nanos() as f64 * savings_ratio) as u128;

        Ok(Duration::from_nanos(savings_nanos as u64))
    }

    fn extract_required_modules(&self, tasks: &[TaskPlan]) -> Vec<String> {
        let mut modules = Vec::new();
        for task in tasks {
            if !modules.contains(&task.module) {
                modules.push(task.module.clone());
            }
        }
        modules
    }

    fn extract_static_files(&self, tasks: &[TaskPlan]) -> Vec<String> {
        let mut files = Vec::new();

        for task in tasks {
            // Extract file references from task arguments
            if let Some(src) = task.args.get("src") {
                if let Some(src_str) = src.as_str() {
                    files.push(src_str.to_string());
                }
            }

            if let Some(content) = task.args.get("content") {
                if content.is_string() {
                    files.push(format!("inline-content-{}", task.task_id));
                }
            }
        }

        files
    }

    fn has_interactive_requirements(&self, task: &TaskPlan) -> bool {
        // Check if task requires interactive input
        task.args.contains_key("prompt")
            || task.args.contains_key("interactive")
            || task.module == "pause"
    }

    fn has_dynamic_arguments(&self, task: &TaskPlan) -> bool {
        // Check if task has Jinja2 templates or variable references
        for value in task.args.values() {
            if let Some(str_val) = value.as_str() {
                if str_val.contains("{{") || str_val.contains("ansible_") {
                    return true;
                }
            }
        }
        false
    }
}

impl Default for BinaryDeploymentAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ModuleRegistry {
    compatibility_map: HashMap<String, ModuleCompatibility>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        let mut compatibility_map = HashMap::new();

        // Define compatibility for common modules
        compatibility_map.insert("debug".to_string(), ModuleCompatibility::FullyCompatible);
        compatibility_map.insert("copy".to_string(), ModuleCompatibility::FullyCompatible);
        compatibility_map.insert("template".to_string(), ModuleCompatibility::FullyCompatible);
        compatibility_map.insert(
            "command".to_string(),
            ModuleCompatibility::PartiallyCompatible {
                limitations: vec!["Command output may vary".to_string()],
            },
        );
        compatibility_map.insert(
            "shell".to_string(),
            ModuleCompatibility::PartiallyCompatible {
                limitations: vec!["Shell environment dependencies".to_string()],
            },
        );
        compatibility_map.insert(
            "package".to_string(),
            ModuleCompatibility::PartiallyCompatible {
                limitations: vec!["Package manager state checks".to_string()],
            },
        );
        compatibility_map.insert(
            "service".to_string(),
            ModuleCompatibility::PartiallyCompatible {
                limitations: vec!["Service state management".to_string()],
            },
        );
        compatibility_map.insert(
            "user".to_string(),
            ModuleCompatibility::Incompatible {
                reasons: vec!["User management requires system access".to_string()],
            },
        );
        compatibility_map.insert(
            "mount".to_string(),
            ModuleCompatibility::Incompatible {
                reasons: vec!["Filesystem operations require root access".to_string()],
            },
        );

        Self { compatibility_map }
    }

    pub fn check_compatibility(
        &self,
        module: &str,
    ) -> Result<ModuleCompatibility, AssessmentError> {
        Ok(self.compatibility_map.get(module).cloned().unwrap_or(
            ModuleCompatibility::PartiallyCompatible {
                limitations: vec!["Unknown module compatibility".to_string()],
            },
        ))
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum ModuleCompatibility {
    FullyCompatible,
    PartiallyCompatible { limitations: Vec<String> },
    Incompatible { reasons: Vec<String> },
}

pub struct ArchitectureDetector {
    default_architecture: String,
}

impl ArchitectureDetector {
    pub fn new() -> Self {
        Self {
            default_architecture: "x86_64-unknown-linux-gnu".to_string(),
        }
    }

    pub fn detect_architecture(&self, hosts: &[String]) -> Result<String, String> {
        // For now, return default architecture
        // In a real implementation, this would query the hosts for their architecture
        if hosts.is_empty() {
            return Err("No hosts provided for architecture detection".to_string());
        }

        // TODO: Implement actual architecture detection via SSH/inventory
        Ok(self.default_architecture.clone())
    }
}

impl Default for ArchitectureDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_task(id: &str, module: &str) -> TaskPlan {
        TaskPlan {
            task_id: id.to_string(),
            name: format!("Test task {id}"),
            module: module.to_string(),
            args: HashMap::new(),
            hosts: vec!["localhost".to_string()],
            dependencies: vec![],
            conditions: vec![],
            tags: vec![],
            notify: vec![],
            execution_order: 0,
            can_run_parallel: true,
            estimated_duration: Duration::from_secs(10),
            risk_level: super::super::rustle_plan::RiskLevel::Low,
        }
    }

    #[test]
    fn test_binary_deployment_analyzer_creation() {
        let _analyzer = BinaryDeploymentAnalyzer::new();
        // Test creation succeeds
    }

    #[test]
    fn test_module_compatibility_check() {
        let registry = ModuleRegistry::new();

        let debug_compat = registry.check_compatibility("debug").unwrap();
        assert!(matches!(debug_compat, ModuleCompatibility::FullyCompatible));

        let user_compat = registry.check_compatibility("user").unwrap();
        assert!(matches!(
            user_compat,
            ModuleCompatibility::Incompatible { .. }
        ));
    }

    #[test]
    fn test_architecture_detection() {
        let detector = ArchitectureDetector::new();
        let hosts = vec!["localhost".to_string()];

        let arch = detector.detect_architecture(&hosts);
        assert!(arch.is_ok());
        assert_eq!(arch.unwrap(), "x86_64-unknown-linux-gnu");
    }

    #[test]
    fn test_binary_compatibility_assessment() {
        let analyzer = BinaryDeploymentAnalyzer::new();
        let task = create_test_task("test1", "debug");

        let compatibility = analyzer.assess_binary_compatibility(&task);
        assert!(compatibility.is_ok());
    }

    #[test]
    fn test_analyze_tasks_below_threshold() {
        let analyzer = BinaryDeploymentAnalyzer::new();
        let tasks = vec![
            create_test_task("task1", "debug"),
            create_test_task("task2", "copy"),
        ];
        let hosts = vec!["localhost".to_string()];

        let deployments = analyzer.analyze_tasks_for_binary_deployment(&tasks, &hosts, 5);
        assert!(deployments.is_ok());
        assert!(deployments.unwrap().is_empty()); // Below threshold
    }

    #[test]
    fn test_analyze_tasks_above_threshold() {
        let analyzer = BinaryDeploymentAnalyzer::new();
        let tasks = vec![
            create_test_task("task1", "debug"),
            create_test_task("task2", "debug"),
            create_test_task("task3", "debug"),
            create_test_task("task4", "debug"),
            create_test_task("task5", "debug"),
        ];
        let hosts = vec!["localhost".to_string()];

        let deployments = analyzer.analyze_tasks_for_binary_deployment(&tasks, &hosts, 3);
        assert!(deployments.is_ok());
        assert!(!deployments.unwrap().is_empty()); // Above threshold
    }
}
