use semver::Version;
use std::collections::{HashMap, HashSet};

use super::compatibility::{RustlePlanParseError, SchemaValidator, ValidationError};
use super::rustle_plan::RustlePlanOutput;

pub struct RustlePlanValidator {
    schema_validator: SchemaValidator,
    min_supported_version: Version,
    max_supported_version: Version,
}

impl RustlePlanValidator {
    pub fn new() -> Result<Self, ValidationError> {
        let schema_validator = SchemaValidator::new().map_err(|e| ValidationError::Schema {
            details: e.to_string(),
        })?;

        Ok(Self {
            schema_validator,
            min_supported_version: Version::parse("0.1.0").unwrap(),
            max_supported_version: Version::parse("1.0.0").unwrap(),
        })
    }

    pub fn validate_rustle_plan(&self, plan: &RustlePlanOutput) -> Result<(), ValidationError> {
        // Validate JSON schema first
        let plan_json = serde_json::to_value(plan).map_err(|e| ValidationError::Schema {
            details: e.to_string(),
        })?;
        self.schema_validator.validate(&plan_json)?;

        // Semantic validation
        self.validate_version_compatibility(plan)?;
        self.validate_plan_structure(plan)?;
        self.validate_task_references(plan)?;
        self.validate_handler_references(plan)?;
        self.validate_dependency_graph(plan)?;
        self.validate_host_consistency(plan)?;

        Ok(())
    }

    fn validate_version_compatibility(
        &self,
        plan: &RustlePlanOutput,
    ) -> Result<(), ValidationError> {
        let version = Version::parse(&plan.metadata.rustle_plan_version).map_err(|e| {
            ValidationError::Semantic {
                field: "metadata.rustle_plan_version".to_string(),
                reason: format!("Invalid version format: {e}"),
            }
        })?;

        if version < self.min_supported_version {
            return Err(ValidationError::Semantic {
                field: "metadata.rustle_plan_version".to_string(),
                reason: format!(
                    "Version {} is too old, minimum supported: {}",
                    version, self.min_supported_version
                ),
            });
        }

        if version > self.max_supported_version {
            return Err(ValidationError::Semantic {
                field: "metadata.rustle_plan_version".to_string(),
                reason: format!(
                    "Version {} is too new, maximum supported: {}",
                    version, self.max_supported_version
                ),
            });
        }

        Ok(())
    }

    fn validate_plan_structure(&self, plan: &RustlePlanOutput) -> Result<(), ValidationError> {
        // Validate that total_tasks matches actual task count
        let actual_task_count: usize = plan
            .plays
            .iter()
            .map(|play| {
                play.batches
                    .iter()
                    .map(|batch| batch.tasks.len())
                    .sum::<usize>()
            })
            .sum();

        if actual_task_count != plan.total_tasks as usize {
            return Err(ValidationError::Semantic {
                field: "total_tasks".to_string(),
                reason: format!(
                    "Total tasks count {} doesn't match actual tasks {}",
                    plan.total_tasks, actual_task_count
                ),
            });
        }

        // Validate that all hosts in plays are included in the global hosts list
        let global_hosts: HashSet<&String> = plan.hosts.iter().collect();
        for (play_idx, play) in plan.plays.iter().enumerate() {
            for host in &play.hosts {
                if !global_hosts.contains(host) {
                    return Err(ValidationError::Semantic {
                        field: format!("plays[{play_idx}].hosts"),
                        reason: format!("Host '{host}' not found in global hosts list"),
                    });
                }
            }
        }

        Ok(())
    }

    fn validate_task_references(&self, plan: &RustlePlanOutput) -> Result<(), ValidationError> {
        // Collect all task IDs
        let mut all_task_ids = HashSet::new();
        for play in &plan.plays {
            for batch in &play.batches {
                for task in &batch.tasks {
                    if !all_task_ids.insert(&task.task_id) {
                        return Err(ValidationError::Reference {
                            reference: task.task_id.clone(),
                            reason: "Duplicate task ID found".to_string(),
                        });
                    }
                }
            }
        }

        // Validate task dependencies
        for play in &plan.plays {
            for batch in &play.batches {
                for task in &batch.tasks {
                    for dep_id in &task.dependencies {
                        if !all_task_ids.contains(dep_id) {
                            return Err(ValidationError::Reference {
                                reference: dep_id.clone(),
                                reason: format!(
                                    "Task dependency '{}' not found for task '{}'",
                                    dep_id, task.task_id
                                ),
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_handler_references(&self, plan: &RustlePlanOutput) -> Result<(), ValidationError> {
        for play in &plan.plays {
            // Collect handler names in this play
            let handler_names: HashSet<&String> = play.handlers.iter().map(|h| &h.name).collect();

            // Validate that all notify references exist
            for batch in &play.batches {
                for task in &batch.tasks {
                    for notify_handler in &task.notify {
                        if !handler_names.contains(notify_handler) {
                            return Err(ValidationError::Reference {
                                reference: notify_handler.clone(),
                                reason: format!(
                                    "Handler '{}' referenced by task '{}' not found in play '{}'",
                                    notify_handler, task.task_id, play.play_id
                                ),
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_dependency_graph(&self, plan: &RustlePlanOutput) -> Result<(), ValidationError> {
        // Build dependency graph and check for cycles
        let mut graph = HashMap::new();
        let mut in_degree = HashMap::new();

        // Initialize graph
        for play in &plan.plays {
            for batch in &play.batches {
                for task in &batch.tasks {
                    graph.insert(task.task_id.clone(), task.dependencies.clone());
                    in_degree.insert(task.task_id.clone(), task.dependencies.len());
                }
            }
        }

        // Topological sort to detect cycles
        let mut queue = Vec::new();
        let mut processed = 0;

        // Find nodes with no incoming edges
        for (task_id, degree) in &in_degree {
            if *degree == 0 {
                queue.push(task_id.clone());
            }
        }

        while let Some(task_id) = queue.pop() {
            processed += 1;

            // Update in-degrees of dependent tasks
            for (other_task, deps) in &graph {
                if deps.contains(&task_id) {
                    if let Some(degree) = in_degree.get_mut(other_task) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(other_task.clone());
                        }
                    }
                }
            }
        }

        if processed != graph.len() {
            return Err(ValidationError::Semantic {
                field: "task_dependencies".to_string(),
                reason: "Circular dependency detected in task graph".to_string(),
            });
        }

        Ok(())
    }

    fn validate_host_consistency(&self, plan: &RustlePlanOutput) -> Result<(), ValidationError> {
        // Validate that task hosts are subsets of their batch/play hosts
        for (play_idx, play) in plan.plays.iter().enumerate() {
            let play_hosts: HashSet<&String> = play.hosts.iter().collect();

            for (batch_idx, batch) in play.batches.iter().enumerate() {
                let batch_hosts: HashSet<&String> = batch.hosts.iter().collect();

                // Batch hosts should be subset of play hosts
                for host in &batch.hosts {
                    if !play_hosts.contains(host) {
                        return Err(ValidationError::Semantic {
                            field: format!("plays[{play_idx}].batches[{batch_idx}].hosts"),
                            reason: format!("Batch host '{host}' not found in play hosts"),
                        });
                    }
                }

                for (task_idx, task) in batch.tasks.iter().enumerate() {
                    // Task hosts should be subset of batch hosts
                    for host in &task.hosts {
                        if !batch_hosts.contains(host) {
                            return Err(ValidationError::Semantic {
                                field: format!("plays[{play_idx}].batches[{batch_idx}].tasks[{task_idx}].hosts"),
                                reason: format!("Task host '{host}' not found in batch hosts"),
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn validate_binary_deployment_plan(
        &self,
        plan: &RustlePlanOutput,
    ) -> Result<(), ValidationError> {
        for (idx, binary_deployment) in plan.binary_deployments.iter().enumerate() {
            // Validate that all referenced task IDs exist
            let all_task_ids: HashSet<String> = plan
                .plays
                .iter()
                .flat_map(|play| play.batches.iter())
                .flat_map(|batch| batch.tasks.iter())
                .map(|task| task.task_id.clone())
                .collect();

            // Check both new format (tasks) and legacy format (task_ids)
            let task_list = if !binary_deployment.tasks.is_empty() {
                &binary_deployment.tasks
            } else if let Some(ref task_ids) = binary_deployment.task_ids {
                task_ids
            } else {
                return Err(ValidationError::Semantic {
                    field: format!("binary_deployments[{idx}].tasks"),
                    reason: "No tasks or task_ids specified in binary deployment".to_string(),
                });
            };

            for task_id in task_list {
                if !all_task_ids.contains(task_id) {
                    return Err(ValidationError::Reference {
                        reference: task_id.clone(),
                        reason: format!(
                            "Binary deployment[{idx}] references non-existent task '{task_id}'"
                        ),
                    });
                }
            }

            // Validate that target hosts are in the global hosts list
            let global_hosts: HashSet<&String> = plan.hosts.iter().collect();
            for host in &binary_deployment.target_hosts {
                if !global_hosts.contains(host) {
                    return Err(ValidationError::Reference {
                        reference: host.clone(),
                        reason: format!("Binary deployment[{idx}] targets unknown host '{host}'"),
                    });
                }
            }

            // Validate target architecture format (handle both new and legacy formats)
            let target_arch = binary_deployment.get_target_architecture();
            if !self.is_valid_target_triple(&target_arch) {
                return Err(ValidationError::Semantic {
                    field: format!("binary_deployments[{idx}].target_architecture"),
                    reason: format!("Invalid target architecture: '{target_arch}'"),
                });
            }
        }

        Ok(())
    }

    fn is_valid_target_triple(&self, target: &str) -> bool {
        // Basic validation for target triple format (arch-vendor-sys)
        let parts: Vec<&str> = target.split('-').collect();
        parts.len() >= 3 && !parts.iter().any(|part| part.is_empty())
    }

    pub fn validate_planning_options(
        &self,
        plan: &RustlePlanOutput,
    ) -> Result<(), ValidationError> {
        let options = &plan.metadata.planning_options;

        // Validate forks is reasonable
        if options.forks == 0 || options.forks > 1000 {
            return Err(ValidationError::Semantic {
                field: "metadata.planning_options.forks".to_string(),
                reason: format!(
                    "Forks value {} is unreasonable (should be 1-1000)",
                    options.forks
                ),
            });
        }

        // Validate binary threshold
        if options.binary_threshold == 0 || options.binary_threshold > 1000 {
            return Err(ValidationError::Semantic {
                field: "metadata.planning_options.binary_threshold".to_string(),
                reason: format!(
                    "Binary threshold {} is unreasonable (should be 1-1000)",
                    options.binary_threshold
                ),
            });
        }

        // Validate that force_binary and force_ssh are not both true
        if options.force_binary && options.force_ssh {
            return Err(ValidationError::Semantic {
                field: "metadata.planning_options".to_string(),
                reason: "Cannot force both binary and SSH execution simultaneously".to_string(),
            });
        }

        Ok(())
    }
}

impl Default for RustlePlanValidator {
    fn default() -> Self {
        Self::new().expect("Failed to create default validator")
    }
}

pub fn validate_rustle_plan_json(
    json_content: &str,
) -> Result<RustlePlanOutput, RustlePlanParseError> {
    // Parse JSON
    let plan: RustlePlanOutput =
        serde_json::from_str(json_content).map_err(|e| RustlePlanParseError::InvalidJson {
            reason: e.to_string(),
        })?;

    // Validate structure
    let validator =
        RustlePlanValidator::new().map_err(|e| RustlePlanParseError::SchemaValidation {
            errors: vec![e.to_string()],
        })?;

    validator
        .validate_rustle_plan(&plan)
        .map_err(|e| RustlePlanParseError::SchemaValidation {
            errors: vec![e.to_string()],
        })?;

    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_minimal_valid_plan() -> RustlePlanOutput {
        use super::super::rustle_plan::*;

        RustlePlanOutput {
            metadata: RustlePlanMetadata {
                created_at: Utc::now(),
                rustle_plan_version: "0.1.0".to_string(),
                playbook_hash: "test-hash".to_string(),
                inventory_hash: "inv-hash".to_string(),
                planning_options: PlanningOptions {
                    limit: None,
                    tags: vec![],
                    skip_tags: vec![],
                    check_mode: false,
                    diff_mode: false,
                    forks: 5,
                    serial: None,
                    strategy: super::super::plan::ExecutionStrategy::Linear,
                    binary_threshold: 3,
                    force_binary: false,
                    force_ssh: false,
                },
            },
            plays: vec![],
            binary_deployments: vec![],
            total_tasks: 0,
            estimated_duration: None,
            estimated_compilation_time: None,
            parallelism_score: 0.0,
            network_efficiency_score: 0.0,
            hosts: vec![],
        }
    }

    #[test]
    fn test_validator_creation() {
        let validator = RustlePlanValidator::new();
        assert!(validator.is_ok());
    }

    #[test]
    fn test_validate_minimal_plan() {
        let validator = RustlePlanValidator::new().unwrap();
        let plan = create_minimal_valid_plan();

        let result = validator.validate_rustle_plan(&plan);
        assert!(result.is_ok());
    }

    #[test]
    fn test_version_compatibility() {
        let validator = RustlePlanValidator::new().unwrap();
        let mut plan = create_minimal_valid_plan();

        // Test unsupported old version
        plan.metadata.rustle_plan_version = "0.0.1".to_string();
        let result = validator.validate_version_compatibility(&plan);
        assert!(result.is_err());

        // Test supported version
        plan.metadata.rustle_plan_version = "0.1.0".to_string();
        let result = validator.validate_version_compatibility(&plan);
        assert!(result.is_ok());
    }

    #[test]
    fn test_target_triple_validation() {
        let validator = RustlePlanValidator::new().unwrap();

        assert!(validator.is_valid_target_triple("x86_64-unknown-linux-gnu"));
        assert!(validator.is_valid_target_triple("aarch64-apple-darwin"));
        assert!(!validator.is_valid_target_triple("invalid"));
        assert!(!validator.is_valid_target_triple("x86_64-"));
        assert!(!validator.is_valid_target_triple(""));
    }

    #[test]
    fn test_planning_options_validation() {
        let validator = RustlePlanValidator::new().unwrap();
        let mut plan = create_minimal_valid_plan();

        // Test invalid forks
        plan.metadata.planning_options.forks = 0;
        let result = validator.validate_planning_options(&plan);
        assert!(result.is_err());

        // Test conflicting force options
        plan.metadata.planning_options.forks = 5;
        plan.metadata.planning_options.force_binary = true;
        plan.metadata.planning_options.force_ssh = true;
        let result = validator.validate_planning_options(&plan);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_json_parsing() {
        let valid_json = r#"{
            "metadata": {
                "created_at": "2025-07-11T05:18:16.945474Z",
                "rustle_plan_version": "0.1.0",
                "playbook_hash": "test",
                "inventory_hash": "test",
                "planning_options": {
                    "limit": null,
                    "tags": [],
                    "skip_tags": [],
                    "check_mode": false,
                    "diff_mode": false,
                    "forks": 5,
                    "serial": null,
                    "strategy": "Linear",
                    "binary_threshold": 3,
                    "force_binary": false,
                    "force_ssh": false
                }
            },
            "plays": [],
            "binary_deployments": [],
            "total_tasks": 0,
            "estimated_duration": null,
            "estimated_compilation_time": null,
            "parallelism_score": 0.0,
            "network_efficiency_score": 0.0,
            "hosts": []
        }"#;

        let result = validate_rustle_plan_json(valid_json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_json_parsing() {
        let invalid_json = "{ invalid json }";

        let result = validate_rustle_plan_json(invalid_json);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RustlePlanParseError::InvalidJson { .. }
        ));
    }
}
