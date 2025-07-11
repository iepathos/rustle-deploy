use anyhow::Result;

use crate::execution::compatibility::AssessmentError;
use crate::execution::rustle_plan::{BinaryCompatibility, TaskPlan};

use super::module_registry::ModuleRegistry;

pub struct BinaryCompatibilityAnalyzer {
    module_registry: ModuleRegistry,
}

impl BinaryCompatibilityAnalyzer {
    pub fn new() -> Self {
        Self {
            module_registry: ModuleRegistry::new(),
        }
    }

    pub fn assess_task_compatibility(
        &self,
        task: &TaskPlan,
    ) -> Result<BinaryCompatibility, AssessmentError> {
        // Check module compatibility first
        let module_compat = self
            .module_registry
            .check_module_compatibility(&task.module)?;

        // Check for task-specific compatibility issues
        let task_limitations = self.analyze_task_limitations(task)?;

        // Combine module and task-level compatibility
        match (module_compat, task_limitations.is_empty()) {
            (BinaryCompatibility::FullyCompatible, true) => {
                Ok(BinaryCompatibility::FullyCompatible)
            }
            (BinaryCompatibility::FullyCompatible, false) => {
                Ok(BinaryCompatibility::PartiallyCompatible {
                    limitations: task_limitations,
                })
            }
            (BinaryCompatibility::PartiallyCompatible { mut limitations }, _) => {
                limitations.extend(task_limitations);
                Ok(BinaryCompatibility::PartiallyCompatible { limitations })
            }
            (BinaryCompatibility::Incompatible { reasons }, _) => {
                Ok(BinaryCompatibility::Incompatible { reasons })
            }
        }
    }

    fn analyze_task_limitations(&self, task: &TaskPlan) -> Result<Vec<String>, AssessmentError> {
        let mut limitations = Vec::new();

        // Check for interactive requirements
        if self.requires_interactive_input(task) {
            limitations.push("Requires interactive user input".to_string());
        }

        // Check for dynamic argument resolution
        if self.has_dynamic_arguments(task) {
            limitations.push("Contains dynamic argument references".to_string());
        }

        // Check for file operations that may not be compatible
        if self.has_complex_file_operations(task) {
            limitations.push("Complex file operations may have limited compatibility".to_string());
        }

        // Check for network operations
        if self.has_network_dependencies(task) {
            limitations.push("Network operations may have connectivity requirements".to_string());
        }

        // Check for system-specific operations
        if self.has_system_dependencies(task) {
            limitations.push("System-specific operations may not be portable".to_string());
        }

        Ok(limitations)
    }

    fn requires_interactive_input(&self, task: &TaskPlan) -> bool {
        // Check module type
        if matches!(task.module.as_str(), "pause" | "prompt" | "expect") {
            return true;
        }

        // Check for interactive arguments
        task.args.contains_key("prompt")
            || task.args.contains_key("interactive")
            || task.args.contains_key("stdin")
    }

    fn has_dynamic_arguments(&self, task: &TaskPlan) -> bool {
        // Check for Jinja2 templating or variable references
        for value in task.args.values() {
            if let Some(str_val) = value.as_str() {
                if str_val.contains("{{")
                    || str_val.contains("ansible_")
                    || str_val.contains("hostvars")
                    || str_val.contains("group_names")
                {
                    return true;
                }
            }
        }
        false
    }

    fn has_complex_file_operations(&self, task: &TaskPlan) -> bool {
        match task.module.as_str() {
            "synchronize" | "unarchive" | "archive" => true,
            "copy" | "template" => {
                // Check for complex copy operations
                task.args.contains_key("remote_src")
                    || task.args.contains_key("backup")
                    || task.args.contains_key("directory_mode")
            }
            _ => false,
        }
    }

    fn has_network_dependencies(&self, task: &TaskPlan) -> bool {
        match task.module.as_str() {
            "uri" | "get_url" | "git" | "subversion" => true,
            "package" => {
                // Package operations typically require network
                !task.args.contains_key("deb") && !task.args.contains_key("rpm")
            }
            _ => {
                // Check for URL references in arguments
                task.args.values().any(|value| {
                    if let Some(str_val) = value.as_str() {
                        str_val.starts_with("http://") || str_val.starts_with("https://")
                    } else {
                        false
                    }
                })
            }
        }
    }

    fn has_system_dependencies(&self, task: &TaskPlan) -> bool {
        match task.module.as_str() {
            "user" | "group" | "mount" | "filesystem" | "lvg" | "lvol" => true,
            "service" | "systemd" => {
                // System service management
                true
            }
            "package" => {
                // Package management requires system package manager
                true
            }
            "command" | "shell" => {
                // Check for system-specific commands
                if let Some(cmd) = task
                    .args
                    .get("_raw_params")
                    .or_else(|| task.args.get("cmd"))
                {
                    if let Some(cmd_str) = cmd.as_str() {
                        // Common system commands that may not be portable
                        let system_commands = [
                            "systemctl",
                            "service",
                            "useradd",
                            "usermod",
                            "groupadd",
                            "mount",
                            "umount",
                            "fdisk",
                            "lsblk",
                            "df",
                            "lsof",
                        ];
                        return system_commands
                            .iter()
                            .any(|sys_cmd| cmd_str.contains(sys_cmd));
                    }
                }
                false
            }
            _ => false,
        }
    }

    pub fn estimate_binary_efficiency(&self, task: &TaskPlan) -> Result<f32, AssessmentError> {
        let compatibility = self.assess_task_compatibility(task)?;

        let base_efficiency = match compatibility {
            BinaryCompatibility::FullyCompatible => 0.9,
            BinaryCompatibility::PartiallyCompatible { ref limitations } => {
                let penalty = limitations.len() as f32 * 0.1;
                (0.7 - penalty).max(0.1)
            }
            BinaryCompatibility::Incompatible { .. } => 0.0,
        };

        // Adjust based on module type
        let module_efficiency = match task.module.as_str() {
            "debug" | "set_fact" | "assert" => 0.95,
            "copy" | "template" => 0.85,
            "command" | "shell" => 0.6,
            "package" | "service" => 0.3,
            _ => 0.5,
        };

        Ok(base_efficiency * module_efficiency)
    }

    pub fn analyze_performance_impact(
        &self,
        tasks: &[TaskPlan],
    ) -> Result<PerformanceAnalysis, AssessmentError> {
        let total_tasks = tasks.len();
        let mut compatible_tasks = 0;
        let mut partially_compatible_tasks = 0;
        let mut incompatible_tasks = 0;
        let mut total_efficiency = 0.0;

        for task in tasks {
            let compatibility = self.assess_task_compatibility(task)?;
            let efficiency = self.estimate_binary_efficiency(task)?;
            total_efficiency += efficiency;

            match compatibility {
                BinaryCompatibility::FullyCompatible => compatible_tasks += 1,
                BinaryCompatibility::PartiallyCompatible { .. } => partially_compatible_tasks += 1,
                BinaryCompatibility::Incompatible { .. } => incompatible_tasks += 1,
            }
        }

        let average_efficiency = if total_tasks > 0 {
            total_efficiency / total_tasks as f32
        } else {
            0.0
        };

        Ok(PerformanceAnalysis {
            total_tasks,
            compatible_tasks,
            partially_compatible_tasks,
            incompatible_tasks,
            average_efficiency,
            recommended_strategy: self.recommend_strategy(
                average_efficiency,
                compatible_tasks,
                total_tasks,
            ),
        })
    }

    fn recommend_strategy(
        &self,
        avg_efficiency: f32,
        compatible_tasks: usize,
        total_tasks: usize,
    ) -> ExecutionStrategy {
        let compatibility_ratio = compatible_tasks as f32 / total_tasks as f32;

        if avg_efficiency > 0.8 && compatibility_ratio > 0.9 {
            ExecutionStrategy::BinaryOnly
        } else if avg_efficiency > 0.5 && compatibility_ratio > 0.6 {
            ExecutionStrategy::BinaryHybrid
        } else {
            ExecutionStrategy::SshOnly
        }
    }
}

impl Default for BinaryCompatibilityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceAnalysis {
    pub total_tasks: usize,
    pub compatible_tasks: usize,
    pub partially_compatible_tasks: usize,
    pub incompatible_tasks: usize,
    pub average_efficiency: f32,
    pub recommended_strategy: ExecutionStrategy,
}

// Re-export ExecutionStrategy for convenience
use crate::execution::plan::ExecutionStrategy;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::Duration;

    fn create_test_task(module: &str, args: HashMap<String, serde_json::Value>) -> TaskPlan {
        use crate::execution::rustle_plan::RiskLevel;

        TaskPlan {
            task_id: "test-task".to_string(),
            name: "Test Task".to_string(),
            module: module.to_string(),
            args,
            hosts: vec!["localhost".to_string()],
            dependencies: vec![],
            conditions: vec![],
            tags: vec![],
            notify: vec![],
            execution_order: 0,
            can_run_parallel: true,
            estimated_duration: Duration::from_secs(10),
            risk_level: RiskLevel::Low,
        }
    }

    #[test]
    fn test_analyzer_creation() {
        let _analyzer = BinaryCompatibilityAnalyzer::new();
        // Just test that creation succeeds
        // Test creation succeeds
    }

    #[test]
    fn test_assess_debug_task_compatibility() {
        let analyzer = BinaryCompatibilityAnalyzer::new();
        let task = create_test_task("debug", HashMap::new());

        let result = analyzer.assess_task_compatibility(&task);
        assert!(result.is_ok());
    }

    #[test]
    fn test_detect_interactive_requirements() {
        let analyzer = BinaryCompatibilityAnalyzer::new();

        let mut args = HashMap::new();
        args.insert(
            "prompt".to_string(),
            serde_json::Value::String("Enter value:".to_string()),
        );
        let task = create_test_task("pause", args);

        assert!(analyzer.requires_interactive_input(&task));
    }

    #[test]
    fn test_detect_dynamic_arguments() {
        let analyzer = BinaryCompatibilityAnalyzer::new();

        let mut args = HashMap::new();
        args.insert(
            "src".to_string(),
            serde_json::Value::String("{{ source_file }}".to_string()),
        );
        let task = create_test_task("copy", args);

        assert!(analyzer.has_dynamic_arguments(&task));
    }

    #[test]
    fn test_estimate_binary_efficiency() {
        let analyzer = BinaryCompatibilityAnalyzer::new();
        let task = create_test_task("debug", HashMap::new());

        let result = analyzer.estimate_binary_efficiency(&task);
        assert!(result.is_ok());
        assert!(result.unwrap() > 0.0);
    }

    #[test]
    fn test_performance_analysis() {
        let analyzer = BinaryCompatibilityAnalyzer::new();
        let tasks = vec![
            create_test_task("debug", HashMap::new()),
            create_test_task("copy", HashMap::new()),
        ];

        let result = analyzer.analyze_performance_impact(&tasks);
        assert!(result.is_ok());

        let analysis = result.unwrap();
        assert_eq!(analysis.total_tasks, 2);
        assert!(analysis.average_efficiency >= 0.0);
    }
}
