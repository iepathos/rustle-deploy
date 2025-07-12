use anyhow::Result;
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

use crate::execution::compatibility::AnalysisError;
use crate::execution::rustle_plan::{BinaryDeploymentPlan, CompilationRequirements, TaskPlan};

use super::analyzer::BinaryCompatibilityAnalyzer;
use super::architecture_detector::ArchitectureDetector;

pub struct BinaryDeploymentPlanner {
    analyzer: BinaryCompatibilityAnalyzer,
    architecture_detector: ArchitectureDetector,
}

impl BinaryDeploymentPlanner {
    pub fn new() -> Self {
        Self {
            analyzer: BinaryCompatibilityAnalyzer::new(),
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

    pub fn create_deployment_plans(
        &self,
        tasks: &[TaskPlan],
        hosts: &[String],
        threshold: u32,
    ) -> Result<Vec<BinaryDeploymentPlan>, AnalysisError> {
        let mut deployment_plans = Vec::new();

        // Group tasks by target architecture
        let architecture_groups = self.group_tasks_by_architecture(tasks, hosts)?;

        for (architecture, task_group) in architecture_groups {
            if task_group.len() >= threshold as usize {
                let deployment_plan =
                    self.create_single_deployment_plan(&task_group, hosts, &architecture)?;
                deployment_plans.push(deployment_plan);
            }
        }

        Ok(deployment_plans)
    }

    fn group_tasks_by_architecture(
        &self,
        tasks: &[TaskPlan],
        hosts: &[String],
    ) -> Result<HashMap<String, Vec<TaskPlan>>, AnalysisError> {
        let mut groups = HashMap::new();

        // For now, assume all hosts have the same architecture
        // In a real implementation, this would query each host
        let architecture = self
            .architecture_detector
            .detect_primary_architecture(hosts)
            .map_err(|_e| AnalysisError::ArchitectureDetection {
                hosts: hosts.to_vec(),
            })?;

        // Filter tasks that are compatible with binary deployment
        for task in tasks {
            match self.analyzer.assess_task_compatibility(task) {
                Ok(compat) => {
                    match compat {
                        crate::execution::rustle_plan::BinaryCompatibility::FullyCompatible |
                        crate::execution::rustle_plan::BinaryCompatibility::PartiallyCompatible { .. } => {
                            groups.entry(architecture.clone())
                                .or_insert_with(Vec::new)
                                .push(task.clone());
                        }
                        crate::execution::rustle_plan::BinaryCompatibility::Incompatible { .. } => {
                            // Skip incompatible tasks
                            continue;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to assess compatibility for task {}: {}",
                        task.task_id,
                        e
                    );
                    continue;
                }
            }
        }

        Ok(groups)
    }

    fn create_single_deployment_plan(
        &self,
        tasks: &[TaskPlan],
        hosts: &[String],
        architecture: &str,
    ) -> Result<BinaryDeploymentPlan, AnalysisError> {
        let deployment_id = format!("binary-{}", Uuid::new_v4());
        let task_ids: Vec<String> = tasks.iter().map(|t| t.task_id.clone()).collect();

        let estimated_savings = self.calculate_time_savings(tasks)?;
        let compilation_requirements = self.build_compilation_requirements(tasks, architecture)?;

        let binary_name = format!("rustle-runner-{deployment_id}");
        let modules: Vec<String> = tasks.iter().map(|t| t.module.clone()).collect();

        Ok(BinaryDeploymentPlan {
            deployment_id,
            target_hosts: hosts.to_vec(),
            binary_name,
            tasks: task_ids.clone(),
            modules,
            embedded_data: Default::default(),
            execution_mode: Default::default(),
            estimated_size: 0,
            compilation_requirements,
            target_architecture: Some(architecture.to_string()),
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
        })
    }

    fn calculate_time_savings(&self, tasks: &[TaskPlan]) -> Result<Duration, AnalysisError> {
        let mut total_estimated_time = Duration::ZERO;
        let mut total_efficiency = 0.0;

        for task in tasks {
            total_estimated_time += task.estimated_duration;

            let efficiency = self
                .analyzer
                .estimate_binary_efficiency(task)
                .map_err(|e| AnalysisError::NetworkEfficiency {
                    reason: format!(
                        "Failed to estimate efficiency for task {}: {}",
                        task.task_id, e
                    ),
                })?;

            total_efficiency += efficiency;
        }

        let average_efficiency = if tasks.is_empty() {
            0.0
        } else {
            total_efficiency / tasks.len() as f32
        };

        // Calculate savings based on efficiency
        let savings_ratio = average_efficiency * 0.4; // Binary deployment can save up to 40% of time
        let savings_nanos = (total_estimated_time.as_nanos() as f64 * savings_ratio as f64) as u128;

        Ok(Duration::from_nanos(
            savings_nanos.min(u64::MAX as u128) as u64
        ))
    }

    fn build_compilation_requirements(
        &self,
        tasks: &[TaskPlan],
        architecture: &str,
    ) -> Result<CompilationRequirements, AnalysisError> {
        let mut modules = Vec::new();
        let mut static_files = Vec::new();
        let mut features = vec!["binary-deployment".to_string()];

        // Extract unique modules
        let mut seen_modules = std::collections::HashSet::new();
        for task in tasks {
            if seen_modules.insert(&task.module) {
                modules.push(task.module.clone());
            }
        }

        // Extract static files referenced in tasks
        for task in tasks {
            static_files.extend(self.extract_static_file_references(task));
        }

        // Add architecture-specific features
        if architecture.contains("linux") {
            features.push("linux-target".to_string());
        }
        if architecture.contains("windows") {
            features.push("windows-target".to_string());
        }
        if architecture.contains("darwin") {
            features.push("macos-target".to_string());
        }

        // Determine optimization level based on task complexity
        let optimization_level = if tasks.len() > 50 {
            "release-lto".to_string() // Link-time optimization for large deployments
        } else if tasks.len() > 10 {
            "release".to_string()
        } else {
            "release-small".to_string() // Optimize for size for small deployments
        };

        Ok(CompilationRequirements {
            target_arch: Self::parse_arch_from_triple(architecture),
            target_os: Self::parse_os_from_triple(architecture),
            rust_version: "1.70.0".to_string(),
            cross_compilation: false,
            static_linking: true,
            modules: Some(modules),
            static_files: Some(static_files),
            target_triple: Some(architecture.to_string()),
            optimization_level: Some(optimization_level),
            features: Some(features),
        })
    }

    fn extract_static_file_references(&self, task: &TaskPlan) -> Vec<String> {
        let mut files = Vec::new();

        // Check common file-related arguments
        if let Some(src) = task.args.get("src") {
            if let Some(src_str) = src.as_str() {
                if !src_str.starts_with("http") && !src_str.contains("{{") {
                    files.push(src_str.to_string());
                }
            }
        }

        if let Some(content) = task.args.get("content") {
            if content.is_string() {
                files.push(format!("inline-content-{}.txt", task.task_id));
            }
        }

        if let Some(template) = task.args.get("template") {
            if let Some(template_str) = template.as_str() {
                files.push(template_str.to_string());
            }
        }

        // Check for file lists in copy/synchronize modules
        if matches!(task.module.as_str(), "copy" | "synchronize") {
            if let Some(files_array) = task.args.get("files") {
                if let Some(files_list) = files_array.as_array() {
                    for file_entry in files_list {
                        if let Some(file_str) = file_entry.as_str() {
                            files.push(file_str.to_string());
                        }
                    }
                }
            }
        }

        files
    }

    pub fn optimize_deployment_strategy(
        &self,
        tasks: &[TaskPlan],
        hosts: &[String],
        constraints: &DeploymentConstraints,
    ) -> Result<OptimizedDeploymentStrategy, AnalysisError> {
        let performance_analysis =
            self.analyzer
                .analyze_performance_impact(tasks)
                .map_err(|e| AnalysisError::NetworkEfficiency {
                    reason: format!("Performance analysis failed: {e}"),
                })?;

        let recommended_threshold =
            self.calculate_optimal_threshold(&performance_analysis, constraints);

        let deployment_plans = self.create_deployment_plans(tasks, hosts, recommended_threshold)?;

        let strategy = if deployment_plans.is_empty() {
            crate::execution::plan::ExecutionStrategy::SshOnly
        } else if performance_analysis.compatible_tasks as f32
            / performance_analysis.total_tasks as f32
            > 0.8
        {
            crate::execution::plan::ExecutionStrategy::BinaryOnly
        } else {
            crate::execution::plan::ExecutionStrategy::BinaryHybrid
        };

        let estimated_total_time = self.estimate_total_execution_time(tasks, &strategy)?;

        Ok(OptimizedDeploymentStrategy {
            strategy,
            binary_deployments: deployment_plans,
            performance_analysis,
            recommended_threshold,
            estimated_total_time,
        })
    }

    fn calculate_optimal_threshold(
        &self,
        analysis: &super::analyzer::PerformanceAnalysis,
        constraints: &DeploymentConstraints,
    ) -> u32 {
        let base_threshold = constraints.min_binary_threshold;

        // Adjust threshold based on compatibility ratio
        let compatibility_ratio = analysis.compatible_tasks as f32 / analysis.total_tasks as f32;

        if compatibility_ratio > 0.9 {
            base_threshold.max(3) // Lower threshold for highly compatible tasks
        } else if compatibility_ratio > 0.7 {
            base_threshold.max(5) // Standard threshold
        } else {
            base_threshold.max(10) // Higher threshold for less compatible tasks
        }
    }

    fn estimate_total_execution_time(
        &self,
        tasks: &[TaskPlan],
        strategy: &crate::execution::plan::ExecutionStrategy,
    ) -> Result<Duration, AnalysisError> {
        let base_time: Duration = tasks.iter().map(|t| t.estimated_duration).sum();

        let efficiency_factor = match strategy {
            crate::execution::plan::ExecutionStrategy::BinaryOnly => 0.6, // 40% time savings
            crate::execution::plan::ExecutionStrategy::BinaryHybrid => 0.8, // 20% time savings
            crate::execution::plan::ExecutionStrategy::SshOnly => 1.0,    // No savings
            _ => 0.9,                                                     // Conservative estimate
        };

        let estimated_time =
            Duration::from_nanos((base_time.as_nanos() as f64 * efficiency_factor) as u64);

        Ok(estimated_time)
    }
}

impl Default for BinaryDeploymentPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct DeploymentConstraints {
    pub min_binary_threshold: u32,
    pub max_compilation_time: Option<Duration>,
    pub target_architectures: Vec<String>,
    pub force_strategy: Option<crate::execution::plan::ExecutionStrategy>,
    pub allow_partial_compatibility: bool,
}

impl Default for DeploymentConstraints {
    fn default() -> Self {
        Self {
            min_binary_threshold: 5,
            max_compilation_time: Some(Duration::from_secs(300)), // 5 minutes
            target_architectures: vec!["x86_64-unknown-linux-gnu".to_string()],
            force_strategy: None,
            allow_partial_compatibility: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OptimizedDeploymentStrategy {
    pub strategy: crate::execution::plan::ExecutionStrategy,
    pub binary_deployments: Vec<BinaryDeploymentPlan>,
    pub performance_analysis: super::analyzer::PerformanceAnalysis,
    pub recommended_threshold: u32,
    pub estimated_total_time: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_task(id: &str, module: &str) -> TaskPlan {
        use crate::execution::rustle_plan::RiskLevel;

        TaskPlan {
            task_id: id.to_string(),
            name: format!("Test Task {id}"),
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
            risk_level: RiskLevel::Low,
        }
    }

    #[test]
    fn test_planner_creation() {
        let _planner = BinaryDeploymentPlanner::new();
        // Test creation succeeds
        // Test creation succeeds
    }

    #[test]
    fn test_create_deployment_plans() {
        let planner = BinaryDeploymentPlanner::new();
        let tasks = vec![
            create_test_task("task1", "debug"),
            create_test_task("task2", "debug"),
            create_test_task("task3", "debug"),
            create_test_task("task4", "debug"),
            create_test_task("task5", "debug"),
        ];
        let hosts = vec!["localhost".to_string()];

        let result = planner.create_deployment_plans(&tasks, &hosts, 3);
        assert!(result.is_ok());
    }

    #[test]
    fn test_calculate_time_savings() {
        let planner = BinaryDeploymentPlanner::new();
        let tasks = vec![
            create_test_task("task1", "debug"),
            create_test_task("task2", "copy"),
        ];

        let result = planner.calculate_time_savings(&tasks);
        assert!(result.is_ok());
        assert!(result.unwrap() > Duration::ZERO);
    }

    #[test]
    fn test_extract_static_file_references() {
        let planner = BinaryDeploymentPlanner::new();
        let mut task = create_test_task("task1", "copy");
        task.args.insert(
            "src".to_string(),
            serde_json::Value::String("/path/to/file".to_string()),
        );

        let files = planner.extract_static_file_references(&task);
        assert!(!files.is_empty());
        assert!(files.contains(&"/path/to/file".to_string()));
    }

    #[test]
    fn test_optimize_deployment_strategy() {
        let planner = BinaryDeploymentPlanner::new();
        let tasks = vec![
            create_test_task("task1", "debug"),
            create_test_task("task2", "debug"),
        ];
        let hosts = vec!["localhost".to_string()];
        let constraints = DeploymentConstraints::default();

        let result = planner.optimize_deployment_strategy(&tasks, &hosts, &constraints);
        assert!(result.is_ok());

        let strategy = result.unwrap();
        assert!(strategy.estimated_total_time > Duration::ZERO);
    }

    #[test]
    fn test_default_constraints() {
        let constraints = DeploymentConstraints::default();
        assert_eq!(constraints.min_binary_threshold, 5);
        assert!(constraints.allow_partial_compatibility);
        assert!(!constraints.target_architectures.is_empty());
    }
}
