use std::collections::HashMap;

use super::binary_analyzer::BinaryDeploymentAnalyzer;
use super::compatibility::ConversionError;
use super::plan::{
    BackoffStrategy, Condition, ConditionOperator, ConnectionConfig, ConnectionMethod,
    DeploymentConfig, ExecutionPlan, ExecutionPlanMetadata, FactDefinition, FactParser,
    FactsTemplate, FailurePolicy, Host, HostGroup, InventoryFormat, InventorySource, InventorySpec,
    ModuleSource, ModuleSpec, RetryPolicy, TargetSelector, Task, TaskType,
};
use super::rustle_plan::{
    BinaryDeploymentPlan, RiskLevel, RustlePlanOutput, TaskCondition, TaskPlan,
};

pub struct RustlePlanConverter {
    binary_analyzer: BinaryDeploymentAnalyzer,
}

impl RustlePlanConverter {
    pub fn new() -> Self {
        Self {
            binary_analyzer: BinaryDeploymentAnalyzer::new(),
        }
    }

    pub fn convert_to_execution_plan(
        &self,
        rustle_plan: &RustlePlanOutput,
    ) -> Result<ExecutionPlan, ConversionError> {
        let mut tasks = Vec::new();

        // Convert play-based structure to flat task list
        for play in &rustle_plan.plays {
            for batch in &play.batches {
                for task in &batch.tasks {
                    let converted_task = self.convert_task(task, play.play_id.as_str())?;
                    tasks.push(converted_task);
                }
            }
        }

        let metadata = self.convert_metadata(rustle_plan)?;
        let inventory = self.construct_inventory_spec(&rustle_plan.hosts)?;
        let strategy = rustle_plan.metadata.planning_options.strategy.clone();
        let facts_template = self.create_default_facts_template();
        let deployment_config = self.create_default_deployment_config();
        let modules = self.extract_module_specs(rustle_plan)?;

        Ok(ExecutionPlan {
            metadata,
            tasks,
            inventory,
            strategy,
            facts_template,
            deployment_config,
            modules,
        })
    }

    pub fn extract_binary_deployments(
        &self,
        rustle_plan: &RustlePlanOutput,
    ) -> Result<Vec<BinaryDeploymentPlan>, ConversionError> {
        // Return existing binary deployments or analyze for new ones
        if !rustle_plan.binary_deployments.is_empty() {
            Ok(rustle_plan.binary_deployments.clone())
        } else {
            // Analyze tasks for binary deployment opportunities
            let all_tasks: Vec<TaskPlan> = rustle_plan
                .plays
                .iter()
                .flat_map(|play| play.batches.iter())
                .flat_map(|batch| batch.tasks.iter())
                .cloned()
                .collect();

            self.binary_analyzer
                .analyze_tasks_for_binary_deployment(
                    &all_tasks,
                    &rustle_plan.hosts,
                    rustle_plan.metadata.planning_options.binary_threshold,
                )
                .map_err(|e| ConversionError::BinaryDeploymentExtraction {
                    reason: e.to_string(),
                })
        }
    }

    fn convert_task(&self, task: &TaskPlan, _play_id: &str) -> Result<Task, ConversionError> {
        let task_type = self.convert_module_to_task_type(&task.module)?;
        let conditions = self.convert_conditions(&task.conditions)?;
        let target_hosts = TargetSelector::Hosts(task.hosts.clone());
        let failure_policy = self.determine_failure_policy(&task.risk_level);

        Ok(Task {
            id: task.task_id.clone(),
            name: task.name.clone(),
            task_type,
            module: task.module.clone(),
            args: task.args.clone(),
            dependencies: task.dependencies.clone(),
            conditions,
            target_hosts,
            timeout: Some(task.estimated_duration),
            retry_policy: self.create_retry_policy(&task.risk_level),
            failure_policy,
        })
    }

    fn convert_metadata(
        &self,
        rustle_plan: &RustlePlanOutput,
    ) -> Result<ExecutionPlanMetadata, ConversionError> {
        Ok(ExecutionPlanMetadata {
            version: "1.0".to_string(),
            created_at: rustle_plan.metadata.created_at,
            rustle_plan_version: rustle_plan.metadata.rustle_version.clone(),
            plan_id: format!("rustle-{}", rustle_plan.metadata.playbook_hash),
            description: None,
            author: None,
            tags: rustle_plan.metadata.planning_options.tags.clone(),
        })
    }

    fn convert_module_to_task_type(&self, module: &str) -> Result<TaskType, ConversionError> {
        match module {
            "command" | "shell" | "script" => Ok(TaskType::Command),
            "copy" | "fetch" => Ok(TaskType::Copy),
            "template" => Ok(TaskType::Template),
            "package" | "yum" | "apt" | "dnf" | "zypper" => Ok(TaskType::Package),
            "service" | "systemd" => Ok(TaskType::Service),
            _ => Ok(TaskType::Custom {
                module_name: module.to_string(),
            }),
        }
    }

    fn convert_conditions(
        &self,
        conditions: &[TaskCondition],
    ) -> Result<Vec<Condition>, ConversionError> {
        let mut converted = Vec::new();

        for condition in conditions {
            match condition {
                TaskCondition::When { expression } => {
                    // Parse simple when expressions
                    if let Some(condition) = self.parse_when_expression(expression) {
                        converted.push(condition);
                    }
                }
                TaskCondition::Tag { tags } => {
                    // Convert tag conditions to existence checks
                    for tag in tags {
                        converted.push(Condition {
                            variable: format!("tags.{tag}"),
                            operator: ConditionOperator::Exists,
                            value: serde_json::Value::Bool(true),
                        });
                    }
                }
                TaskCondition::Only { hosts } => {
                    // Convert to host-based condition
                    converted.push(Condition {
                        variable: "inventory_hostname".to_string(),
                        operator: ConditionOperator::Contains,
                        value: serde_json::Value::Array(
                            hosts
                                .iter()
                                .map(|h| serde_json::Value::String(h.clone()))
                                .collect(),
                        ),
                    });
                }
                TaskCondition::Skip { condition } => {
                    // Convert skip conditions to negated when conditions
                    if let Some(mut skip_condition) = self.parse_when_expression(condition) {
                        // Negate the operator
                        skip_condition.operator = match skip_condition.operator {
                            ConditionOperator::Equals => ConditionOperator::NotEquals,
                            ConditionOperator::NotEquals => ConditionOperator::Equals,
                            ConditionOperator::Exists => ConditionOperator::NotExists,
                            ConditionOperator::NotExists => ConditionOperator::Exists,
                            other => other, // Keep others as-is
                        };
                        converted.push(skip_condition);
                    }
                }
            }
        }

        Ok(converted)
    }

    fn parse_when_expression(&self, expression: &str) -> Option<Condition> {
        // Parse simple expressions like "var is defined", "var == 'value'", etc.
        let trimmed = expression.trim();

        if trimmed.ends_with("is defined") {
            let var_name = trimmed.strip_suffix("is defined")?.trim();
            return Some(Condition {
                variable: var_name.to_string(),
                operator: ConditionOperator::Exists,
                value: serde_json::Value::Bool(true),
            });
        }

        if trimmed.ends_with("is not defined") {
            let var_name = trimmed.strip_suffix("is not defined")?.trim();
            return Some(Condition {
                variable: var_name.to_string(),
                operator: ConditionOperator::NotExists,
                value: serde_json::Value::Bool(true),
            });
        }

        if let Some(eq_pos) = trimmed.find(" == ") {
            let var_name = trimmed[..eq_pos].trim();
            let value_str = trimmed[eq_pos + 4..]
                .trim()
                .trim_matches('"')
                .trim_matches('\'');
            return Some(Condition {
                variable: var_name.to_string(),
                operator: ConditionOperator::Equals,
                value: serde_json::Value::String(value_str.to_string()),
            });
        }

        if let Some(ne_pos) = trimmed.find(" != ") {
            let var_name = trimmed[..ne_pos].trim();
            let value_str = trimmed[ne_pos + 4..]
                .trim()
                .trim_matches('"')
                .trim_matches('\'');
            return Some(Condition {
                variable: var_name.to_string(),
                operator: ConditionOperator::NotEquals,
                value: serde_json::Value::String(value_str.to_string()),
            });
        }

        // Default: treat the whole expression as a variable existence check
        Some(Condition {
            variable: trimmed.to_string(),
            operator: ConditionOperator::Exists,
            value: serde_json::Value::Bool(true),
        })
    }

    fn determine_failure_policy(&self, risk_level: &RiskLevel) -> FailurePolicy {
        match risk_level {
            RiskLevel::Low => FailurePolicy::Continue,
            RiskLevel::Medium => FailurePolicy::Continue,
            RiskLevel::High => FailurePolicy::Abort,
            RiskLevel::Critical => FailurePolicy::Rollback,
        }
    }

    fn create_retry_policy(&self, risk_level: &RiskLevel) -> Option<RetryPolicy> {
        match risk_level {
            RiskLevel::Low => Some(RetryPolicy {
                max_attempts: 3,
                delay: std::time::Duration::from_secs(1),
                backoff: BackoffStrategy::Linear,
            }),
            RiskLevel::Medium => Some(RetryPolicy {
                max_attempts: 2,
                delay: std::time::Duration::from_secs(2),
                backoff: BackoffStrategy::Exponential,
            }),
            RiskLevel::High => Some(RetryPolicy {
                max_attempts: 1,
                delay: std::time::Duration::from_secs(5),
                backoff: BackoffStrategy::Fixed,
            }),
            RiskLevel::Critical => None, // No retries for critical tasks
        }
    }

    fn construct_inventory_spec(&self, hosts: &[String]) -> Result<InventorySpec, ConversionError> {
        let mut inventory_hosts = HashMap::new();
        let mut groups = HashMap::new();

        // Create default host entries
        for host in hosts {
            inventory_hosts.insert(
                host.clone(),
                Host {
                    address: host.clone(),
                    connection: ConnectionConfig {
                        method: if host == "localhost" {
                            ConnectionMethod::Local
                        } else {
                            ConnectionMethod::Ssh
                        },
                        username: None,
                        password: None,
                        key_file: None,
                        port: None,
                        timeout: Some(std::time::Duration::from_secs(30)),
                    },
                    variables: HashMap::new(),
                    target_triple: Some("x86_64-unknown-linux-gnu".to_string()),
                },
            );
        }

        // Create an "all" group containing all hosts
        groups.insert(
            "all".to_string(),
            HostGroup {
                hosts: hosts.to_vec(),
                variables: HashMap::new(),
                children: vec![],
            },
        );

        Ok(InventorySpec {
            format: InventoryFormat::Dynamic,
            source: InventorySource::Inline {
                content: "# Generated from rustle-plan output".to_string(),
            },
            groups,
            hosts: inventory_hosts,
            variables: HashMap::new(),
        })
    }

    fn create_default_facts_template(&self) -> FactsTemplate {
        let mut custom_facts = HashMap::new();

        custom_facts.insert(
            "ansible_facts".to_string(),
            FactDefinition {
                command: "ansible -m setup localhost".to_string(),
                parser: FactParser::Json,
                cache_ttl: Some(std::time::Duration::from_secs(300)),
            },
        );

        FactsTemplate {
            global_facts: vec!["ansible_facts".to_string()],
            host_facts: vec![
                "ansible_hostname".to_string(),
                "ansible_architecture".to_string(),
            ],
            custom_facts,
        }
    }

    fn create_default_deployment_config(&self) -> DeploymentConfig {
        DeploymentConfig {
            target_path: "/tmp/rustle-deploy".to_string(),
            backup_previous: true,
            verify_deployment: true,
            cleanup_on_success: false,
            deployment_timeout: Some(std::time::Duration::from_secs(1800)), // 30 minutes
        }
    }

    fn extract_module_specs(
        &self,
        rustle_plan: &RustlePlanOutput,
    ) -> Result<Vec<ModuleSpec>, ConversionError> {
        let mut modules = Vec::new();
        let mut seen_modules = std::collections::HashSet::new();

        // Extract unique modules from all tasks
        for play in &rustle_plan.plays {
            for batch in &play.batches {
                for task in &batch.tasks {
                    if seen_modules.insert(task.module.clone()) {
                        modules.push(ModuleSpec {
                            name: task.module.clone(),
                            source: self.determine_module_source(&task.module),
                            version: None,
                            checksum: None,
                            dependencies: vec![],
                            static_link: self.should_static_link(&task.module),
                        });
                    }
                }
            }
        }

        Ok(modules)
    }

    fn determine_module_source(&self, module: &str) -> ModuleSource {
        match module {
            "debug" | "copy" | "template" | "command" | "shell" | "service" | "package" => {
                ModuleSource::Builtin
            }
            _ => ModuleSource::Registry {
                name: module.to_string(),
                version: "latest".to_string(),
            },
        }
    }

    fn should_static_link(&self, module: &str) -> bool {
        // Determine if module should be statically linked for binary deployment
        match module {
            "debug" | "copy" | "template" | "command" => true,
            "service" | "package" | "user" | "mount" => false, // These require system integration
            _ => false,                                        // Conservative default
        }
    }
}

impl Default for RustlePlanConverter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::time::Duration;

    fn create_test_rustle_plan() -> RustlePlanOutput {
        use super::super::plan::ExecutionStrategy;
        use super::super::rustle_plan::*;

        RustlePlanOutput {
            metadata: RustlePlanMetadata {
                created_at: Utc::now(),
                rustle_version: "0.1.0".to_string(),
                playbook_hash: "test-hash".to_string(),
                inventory_hash: "inv-hash".to_string(),
                planning_options: PlanningOptions {
                    limit: None,
                    tags: vec!["test".to_string()],
                    skip_tags: vec![],
                    check_mode: false,
                    diff_mode: false,
                    forks: 5,
                    serial: None,
                    strategy: ExecutionStrategy::Linear,
                    binary_threshold: 3,
                    force_binary: false,
                    force_ssh: false,
                },
            },
            plays: vec![PlayPlan {
                play_id: "play-1".to_string(),
                name: "Test Play".to_string(),
                strategy: ExecutionStrategy::Linear,
                serial: None,
                hosts: vec!["localhost".to_string()],
                batches: vec![TaskBatch {
                    batch_id: "batch-1".to_string(),
                    hosts: vec!["localhost".to_string()],
                    tasks: vec![TaskPlan {
                        task_id: "task-1".to_string(),
                        name: "Test Task".to_string(),
                        module: "debug".to_string(),
                        args: {
                            let mut args = HashMap::new();
                            args.insert(
                                "msg".to_string(),
                                serde_json::Value::String("Hello".to_string()),
                            );
                            args
                        },
                        hosts: vec!["localhost".to_string()],
                        dependencies: vec![],
                        conditions: vec![],
                        tags: vec!["test".to_string()],
                        notify: vec![],
                        execution_order: 0,
                        can_run_parallel: true,
                        estimated_duration: Duration::from_secs(1),
                        risk_level: RiskLevel::Low,
                    }],
                    parallel_groups: vec![],
                    dependencies: vec![],
                    estimated_duration: None,
                }],
                handlers: vec![],
                estimated_duration: None,
            }],
            binary_deployments: vec![],
            total_tasks: 1,
            estimated_duration: Some(Duration::from_secs(10)),
            estimated_compilation_time: None,
            parallelism_score: 0.5,
            network_efficiency_score: 0.3,
            hosts: vec!["localhost".to_string()],
        }
    }

    #[test]
    fn test_convert_execution_plan() {
        let converter = RustlePlanConverter::new();
        let rustle_plan = create_test_rustle_plan();

        let result = converter.convert_to_execution_plan(&rustle_plan);
        assert!(result.is_ok());

        let execution_plan = result.unwrap();
        assert_eq!(execution_plan.tasks.len(), 1);
        assert_eq!(execution_plan.tasks[0].name, "Test Task");
        assert_eq!(execution_plan.tasks[0].module, "debug");
    }

    #[test]
    fn test_convert_task() {
        let converter = RustlePlanConverter::new();
        let task_plan = TaskPlan {
            task_id: "test-1".to_string(),
            name: "Test Task".to_string(),
            module: "debug".to_string(),
            args: HashMap::new(),
            hosts: vec!["localhost".to_string()],
            dependencies: vec![],
            conditions: vec![],
            tags: vec![],
            notify: vec![],
            execution_order: 0,
            can_run_parallel: true,
            estimated_duration: Duration::from_secs(5),
            risk_level: RiskLevel::Medium,
        };

        let result = converter.convert_task(&task_plan, "play-1");
        assert!(result.is_ok());

        let task = result.unwrap();
        assert_eq!(task.id, "test-1");
        assert_eq!(task.name, "Test Task");
        assert!(matches!(task.task_type, TaskType::Custom { .. }));
    }

    #[test]
    fn test_parse_when_expression() {
        let converter = RustlePlanConverter::new();

        let condition = converter.parse_when_expression("test_var is defined");
        assert!(condition.is_some());
        let cond = condition.unwrap();
        assert_eq!(cond.variable, "test_var");
        assert!(matches!(cond.operator, ConditionOperator::Exists));

        let condition = converter.parse_when_expression("var == 'value'");
        assert!(condition.is_some());
        let cond = condition.unwrap();
        assert_eq!(cond.variable, "var");
        assert!(matches!(cond.operator, ConditionOperator::Equals));
    }

    #[test]
    fn test_determine_failure_policy() {
        let converter = RustlePlanConverter::new();

        assert!(matches!(
            converter.determine_failure_policy(&RiskLevel::Low),
            FailurePolicy::Continue
        ));
        assert!(matches!(
            converter.determine_failure_policy(&RiskLevel::Critical),
            FailurePolicy::Rollback
        ));
    }

    #[test]
    fn test_construct_inventory_spec() {
        let converter = RustlePlanConverter::new();
        let hosts = vec!["localhost".to_string(), "remote".to_string()];

        let result = converter.construct_inventory_spec(&hosts);
        assert!(result.is_ok());

        let inventory = result.unwrap();
        assert_eq!(inventory.hosts.len(), 2);
        assert!(inventory.hosts.contains_key("localhost"));
        assert!(inventory.hosts.contains_key("remote"));
        assert!(inventory.groups.contains_key("all"));
    }
}
