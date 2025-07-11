use crate::execution::{ExecutionPlan, Task};
use crate::modules::{ExecutionContext, HostInfo, ModuleArgs, ModuleRegistry, SpecialParameters};
use crate::runtime::{
    conditions::{ConditionContext, ConditionEvaluator},
    error::{CleanupError, ExecutionError},
    facts::FactsCache,
    progress::ProgressReporter,
    state::{ExecutionResult, StateManager, TaskResult, TaskStatus},
};
use chrono::Utc;
use petgraph::{algo::toposort, Graph};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Runtime configuration for the executor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub controller_endpoint: Option<String>,
    #[serde(with = "serde_duration")]
    pub execution_timeout: Duration,
    #[serde(with = "serde_duration_opt")]
    pub task_timeout: Option<Duration>,
    #[serde(with = "serde_duration")]
    pub report_interval: Duration,
    pub cleanup_on_completion: bool,
    pub log_level: String,
    pub check_mode: Option<bool>,
    pub parallel_tasks: Option<usize>,
    #[serde(with = "serde_duration")]
    pub facts_cache_ttl: Duration,
    pub retry_policy: Option<RetryPolicyConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicyConfig {
    pub max_attempts: u32,
    #[serde(with = "serde_duration")]
    pub delay: Duration,
    pub backoff: BackoffStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Fixed,
    Linear,
    Exponential,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            controller_endpoint: None,
            execution_timeout: Duration::from_secs(3600), // 1 hour
            task_timeout: Some(Duration::from_secs(300)), // 5 minutes
            report_interval: Duration::from_secs(5),
            cleanup_on_completion: true,
            log_level: "info".to_string(),
            check_mode: Some(false),
            parallel_tasks: Some(4),
            facts_cache_ttl: Duration::from_secs(300), // 5 minutes
            retry_policy: None,
        }
    }
}

/// Main execution engine for embedded execution plans
pub struct LocalExecutor {
    config: RuntimeConfig,
    module_registry: ModuleRegistry,
    facts_cache: FactsCache,
    state_manager: StateManager,
    progress_reporter: ProgressReporter,
    execution_id: String,
}

impl LocalExecutor {
    pub fn new(config: RuntimeConfig) -> Self {
        let execution_id = Uuid::new_v4().to_string();
        let facts_cache = FactsCache::new(config.facts_cache_ttl);
        let progress_reporter = ProgressReporter::new(config.controller_endpoint.clone());

        Self {
            module_registry: ModuleRegistry::with_core_modules(),
            state_manager: StateManager::new(execution_id.clone(), 0), // Will be updated when plan is loaded
            facts_cache,
            progress_reporter,
            execution_id,
            config,
        }
    }

    /// Execute a complete execution plan
    pub async fn execute_plan(
        &mut self,
        plan: ExecutionPlan,
    ) -> Result<ExecutionResult, ExecutionError> {
        let start_time = Instant::now();

        // Initialize state manager with correct task count
        self.state_manager = StateManager::new(self.execution_id.clone(), plan.tasks.len());

        tracing::info!("Starting execution of plan with {} tasks", plan.tasks.len());

        // Report execution start
        self.progress_reporter
            .report_execution_start(&self.execution_id, plan.tasks.len())
            .await?;

        // Collect and cache facts
        if let Err(e) = self.collect_facts() {
            tracing::warn!("Failed to collect facts: {}", e);
        }

        // Execute all tasks
        let result = match self.execute_tasks(&plan.tasks).await {
            Ok(_) => {
                let end_time = Utc::now();
                self.state_manager.build_execution_result(end_time)
            }
            Err(e) => {
                tracing::error!("Execution failed: {}", e);
                self.progress_reporter
                    .report_error(&self.execution_id, &e.to_string())
                    .await?;

                let end_time = Utc::now();
                let mut result = self.state_manager.build_execution_result(end_time);
                result.failed = true;
                result.errors.push(e.to_string());
                result
            }
        };

        // Report execution completion
        self.progress_reporter
            .report_execution_complete(&result)
            .await?;

        // Cleanup if configured
        if self.config.cleanup_on_completion {
            if let Err(e) = self.cleanup() {
                tracing::warn!("Cleanup failed: {}", e);
            }
        }

        tracing::info!(
            "Execution completed in {:?}: {} tasks, {} failed, {} changed",
            start_time.elapsed(),
            result.summary.total_tasks,
            result.summary.failed_tasks,
            result.summary.changed_tasks
        );

        Ok(result)
    }

    /// Execute tasks with dependency resolution
    async fn execute_tasks(&mut self, tasks: &[Task]) -> Result<(), ExecutionError> {
        if tasks.is_empty() {
            return Ok(());
        }

        // Build dependency graph
        let dependency_graph = self.build_dependency_graph(tasks)?;

        // Execute tasks in dependency order
        let mut completed = HashSet::new();
        let mut failed = HashSet::new();

        while completed.len() + failed.len() < tasks.len() {
            let ready_tasks = self.find_ready_tasks(tasks, &dependency_graph, &completed, &failed);

            if ready_tasks.is_empty() {
                let remaining: Vec<String> = tasks
                    .iter()
                    .filter(|t| !completed.contains(&t.id) && !failed.contains(&t.id))
                    .map(|t| t.id.clone())
                    .collect();

                return Err(ExecutionError::DependencyCycle { cycle: remaining });
            }

            // Execute ready tasks (potentially in parallel)
            let max_parallel = self.config.parallel_tasks.unwrap_or(1);
            for chunk in ready_tasks.chunks(max_parallel) {
                let mut results = Vec::new();
                for task in chunk {
                    let result = self.execute_task(task).await;
                    results.push(result);
                }

                for result in results {
                    match result {
                        Ok(task_result) => {
                            let task_id = task_result.task_id.clone();
                            if task_result.failed {
                                failed.insert(task_id.clone());
                            } else {
                                completed.insert(task_id.clone());
                            }
                            self.state_manager.add_task_result(task_result);
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute a single task
    pub async fn execute_task(&mut self, task: &Task) -> Result<TaskResult, ExecutionError> {
        let start_time = Instant::now();
        let start_utc = Utc::now();

        tracing::debug!("Executing task: {} ({})", task.name, task.id);

        // Update current task in state
        self.state_manager.set_current_task(Some(task.id.clone()));

        // Report task start
        self.progress_reporter.report_task_start(task).await?;

        // Evaluate conditions
        let condition_context = ConditionContext::new(
            self.facts_cache.get_all_facts(),
            HashMap::new(), // TODO: Get variables from execution context
            self.state_manager.get_all_task_results().clone(),
        );

        if !ConditionEvaluator::evaluate_conditions(&task.conditions, &condition_context)? {
            let result = TaskResult {
                task_id: task.id.clone(),
                name: task.name.clone(),
                status: TaskStatus::Skipped,
                changed: false,
                failed: false,
                skipped: true,
                output: serde_json::json!({"skipped": true, "reason": "Condition not met"}),
                stdout: None,
                stderr: None,
                start_time: start_utc,
                end_time: Utc::now(),
                duration: start_time.elapsed(),
                error: None,
            };

            self.progress_reporter.report_task_complete(&result).await?;
            return Ok(result);
        }

        // Prepare execution context
        let execution_context = ExecutionContext {
            facts: self.facts_cache.get_all_facts(),
            variables: HashMap::new(), // TODO: Populate from execution plan
            host_info: HostInfo::detect(),
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            environment: std::env::vars().collect(),
            check_mode: self.config.check_mode.unwrap_or(false),
            diff_mode: false,
            verbosity: 0,
        };

        // Prepare module arguments
        let module_args = ModuleArgs {
            args: task.args.clone(),
            special: SpecialParameters {
                r#become: None, // TODO: Extract from task
                when: None,
                changed_when: None,
                failed_when: None,
                check_mode: execution_context.check_mode,
                diff: execution_context.diff_mode,
            },
        };

        // Execute the task with timeout
        let module_result = match (
            task.timeout.or(self.config.task_timeout),
            &task.retry_policy,
        ) {
            (Some(timeout), Some(retry)) => {
                self.execute_with_retry(
                    &task.module,
                    &module_args,
                    &execution_context,
                    timeout,
                    retry,
                )
                .await?
            }
            (Some(timeout), None) => {
                tokio::time::timeout(
                    timeout,
                    self.module_registry.execute_module(
                        &task.module,
                        &module_args,
                        &execution_context,
                    ),
                )
                .await??
            }
            (None, Some(retry)) => {
                self.execute_with_retry(
                    &task.module,
                    &module_args,
                    &execution_context,
                    Duration::from_secs(300),
                    retry,
                )
                .await?
            }
            (None, None) => {
                self.module_registry
                    .execute_module(&task.module, &module_args, &execution_context)
                    .await?
            }
        };

        let end_utc = Utc::now();
        let task_result = TaskResult {
            task_id: task.id.clone(),
            name: task.name.clone(),
            status: if module_result.failed {
                TaskStatus::Failed
            } else {
                TaskStatus::Success
            },
            changed: module_result.changed,
            failed: module_result.failed,
            skipped: false,
            output: serde_json::to_value(&module_result.results)?,
            stdout: module_result.stdout,
            stderr: module_result.stderr,
            start_time: start_utc,
            end_time: end_utc,
            duration: start_time.elapsed(),
            error: if module_result.failed {
                Some(
                    module_result
                        .msg
                        .unwrap_or_else(|| "Task failed".to_string()),
                )
            } else {
                None
            },
        };

        // Report task completion
        self.progress_reporter
            .report_task_complete(&task_result)
            .await?;

        tracing::debug!(
            "Task completed: {} - {} in {:?}",
            task.name,
            if task_result.failed {
                "FAILED"
            } else if task_result.changed {
                "CHANGED"
            } else {
                "OK"
            },
            task_result.duration
        );

        Ok(task_result)
    }

    async fn execute_with_retry(
        &self,
        module_name: &str,
        args: &ModuleArgs,
        context: &ExecutionContext,
        timeout: Duration,
        retry_policy: &crate::execution::RetryPolicy,
    ) -> Result<crate::modules::ModuleResult, ExecutionError> {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts < retry_policy.max_attempts {
            attempts += 1;

            let result = tokio::time::timeout(
                timeout,
                self.module_registry
                    .execute_module(module_name, args, context),
            )
            .await;

            match result {
                Ok(Ok(module_result)) => {
                    if !module_result.failed {
                        return Ok(module_result);
                    }
                    last_error = Some(ExecutionError::ModuleExecution(
                        crate::modules::ModuleExecutionError::ExecutionFailed {
                            message: module_result
                                .msg
                                .unwrap_or_else(|| "Task failed".to_string()),
                        },
                    ));
                }
                Ok(Err(e)) => {
                    last_error = Some(ExecutionError::ModuleExecution(e));
                }
                Err(e) => {
                    last_error = Some(ExecutionError::Timeout(e));
                }
            }

            if attempts < retry_policy.max_attempts {
                let delay = self.calculate_retry_delay(retry_policy, attempts);
                tracing::debug!(
                    "Task failed, retrying in {:?} (attempt {}/{})",
                    delay,
                    attempts,
                    retry_policy.max_attempts
                );
                tokio::time::sleep(delay).await;
            }
        }

        Err(last_error.unwrap_or_else(|| ExecutionError::TaskFailed {
            task_id: "unknown".to_string(),
            reason: "All retry attempts failed".to_string(),
        }))
    }

    fn calculate_retry_delay(
        &self,
        retry_policy: &crate::execution::RetryPolicy,
        attempt: u32,
    ) -> Duration {
        match retry_policy.backoff {
            crate::execution::BackoffStrategy::Fixed => retry_policy.delay,
            crate::execution::BackoffStrategy::Linear => retry_policy.delay * attempt,
            crate::execution::BackoffStrategy::Exponential => {
                retry_policy.delay * (2_u32.pow(attempt - 1))
            }
        }
    }

    fn build_dependency_graph(
        &self,
        tasks: &[Task],
    ) -> Result<HashMap<String, Vec<String>>, ExecutionError> {
        let mut graph = HashMap::new();
        let task_ids: HashSet<String> = tasks.iter().map(|t| t.id.clone()).collect();

        for task in tasks {
            // Validate that all dependencies exist
            for dep in &task.dependencies {
                if !task_ids.contains(dep) {
                    return Err(ExecutionError::InvalidExecutionPlan {
                        reason: format!(
                            "Task '{}' depends on non-existent task '{}'",
                            task.id, dep
                        ),
                    });
                }
            }
            graph.insert(task.id.clone(), task.dependencies.clone());
        }

        // Check for cycles using petgraph
        let mut petgraph = Graph::new();
        let mut node_map = HashMap::new();

        // Add nodes
        for task in tasks {
            let node_idx = petgraph.add_node(task.id.clone());
            node_map.insert(task.id.clone(), node_idx);
        }

        // Add edges
        for task in tasks {
            if let Some(task_node) = node_map.get(&task.id) {
                for dep in &task.dependencies {
                    if let Some(dep_node) = node_map.get(dep) {
                        petgraph.add_edge(*dep_node, *task_node, ());
                    }
                }
            }
        }

        // Check for cycles
        if toposort(&petgraph, None).is_err() {
            return Err(ExecutionError::DependencyCycle {
                cycle: tasks.iter().map(|t| t.id.clone()).collect(),
            });
        }

        Ok(graph)
    }

    fn find_ready_tasks<'a>(
        &self,
        tasks: &'a [Task],
        dependency_graph: &HashMap<String, Vec<String>>,
        completed: &HashSet<String>,
        failed: &HashSet<String>,
    ) -> Vec<&'a Task> {
        tasks
            .iter()
            .filter(|task| {
                // Task is not already completed or failed
                !completed.contains(&task.id) && !failed.contains(&task.id)
            })
            .filter(|task| {
                // All dependencies are completed
                dependency_graph
                    .get(&task.id)
                    .map(|deps| deps.iter().all(|dep| completed.contains(dep)))
                    .unwrap_or(true)
            })
            .collect()
    }

    /// Collect system facts
    pub fn collect_facts(&mut self) -> Result<(), ExecutionError> {
        self.facts_cache
            .refresh_facts()
            .map_err(|e| ExecutionError::FactsCollectionFailed {
                reason: e.to_string(),
            })?;

        let facts = self.facts_cache.get_all_facts();
        self.state_manager.set_facts(facts);

        Ok(())
    }

    /// Clean up resources
    pub fn cleanup(&self) -> Result<(), CleanupError> {
        tracing::debug!("Cleaning up execution resources");
        // TODO: Implement cleanup logic (temporary files, etc.)
        Ok(())
    }
}

// Custom serialization for Duration fields
mod serde_duration {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

mod serde_duration_opt {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match duration {
            Some(d) => Some(d.as_secs()).serialize(serializer),
            None => None::<u64>.serialize(serializer),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs_opt = Option::<u64>::deserialize(deserializer)?;
        Ok(secs_opt.map(Duration::from_secs))
    }
}
