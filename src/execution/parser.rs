use crate::execution::{
    DependencyError, ExecutionPlan, ExtractionError, OrderingError, ParseError, TemplateError,
    ValidationError,
};
use crate::types::DeploymentTarget;
use serde_json;
use serde_yaml;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub enum PlanFormat {
    Json,
    Yaml,
    Auto,
}

pub struct ExecutionPlanParser {
    schema_validator: SchemaValidator,
    template_processor: TemplateProcessor,
}

impl ExecutionPlanParser {
    pub fn new() -> Self {
        Self {
            schema_validator: SchemaValidator::new(),
            template_processor: TemplateProcessor::new(),
        }
    }

    pub fn parse(&self, content: &str, format: PlanFormat) -> Result<ExecutionPlan, ParseError> {
        let detected_format = match format {
            PlanFormat::Auto => self.detect_format(content)?,
            format => format,
        };

        let plan: ExecutionPlan = match detected_format {
            PlanFormat::Json => {
                serde_json::from_str(content).map_err(|e| ParseError::InvalidJson {
                    reason: e.to_string(),
                })?
            }
            PlanFormat::Yaml => {
                serde_yaml::from_str(content).map_err(|e| ParseError::InvalidYaml {
                    reason: e.to_string(),
                })?
            }
            PlanFormat::Auto => unreachable!("Auto format should be resolved by now"),
        };

        self.validate(&plan)?;
        Ok(plan)
    }

    pub fn validate(&self, plan: &ExecutionPlan) -> Result<(), ValidationError> {
        self.schema_validator.validate_plan(plan)?;
        self.validate_dependencies(plan)?;
        Ok(())
    }

    pub fn resolve_templates(
        &self,
        plan: &ExecutionPlan,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<ExecutionPlan, TemplateError> {
        self.template_processor.process_plan(plan, variables)
    }

    pub fn extract_deployment_targets(
        &self,
        plan: &ExecutionPlan,
    ) -> Result<Vec<DeploymentTarget>, ExtractionError> {
        let mut targets = Vec::new();

        // Extract hosts from inventory
        for host in plan.inventory.hosts.values() {
            let target_triple = host
                .target_triple
                .clone()
                .unwrap_or_else(|| "x86_64-unknown-linux-gnu".to_string());

            targets.push(DeploymentTarget {
                host: host.address.clone(),
                target_path: plan.deployment_config.target_path.clone(),
                binary_compilation_id: format!("rustle-{target_triple}"),
                deployment_method: crate::types::DeploymentMethod::Ssh,
                status: crate::types::DeploymentStatus::Pending,
                deployed_at: None,
                version: "1.0.0".to_string(),
            });
        }

        if targets.is_empty() {
            return Err(ExtractionError::ExtractionFailed {
                reason: "No valid deployment targets found in inventory".to_string(),
            });
        }

        Ok(targets)
    }

    pub fn validate_dependencies(&self, plan: &ExecutionPlan) -> Result<(), DependencyError> {
        // Build dependency graph
        let mut task_map = HashMap::new();
        for task in &plan.tasks {
            task_map.insert(task.id.clone(), task);
        }

        // Check that all dependencies exist
        for task in &plan.tasks {
            for dep_id in &task.dependencies {
                if !task_map.contains_key(dep_id) {
                    return Err(DependencyError::MissingDependency {
                        missing: dep_id.clone(),
                    });
                }
            }
        }

        // Check for circular dependencies
        self.check_circular_dependencies(&plan.tasks)?;

        Ok(())
    }

    pub fn compute_execution_order(
        &self,
        plan: &ExecutionPlan,
    ) -> Result<Vec<String>, OrderingError> {
        use std::collections::{HashMap, VecDeque};

        let mut graph = HashMap::new();
        let mut in_degree = HashMap::new();

        // Build dependency graph
        for task in &plan.tasks {
            graph.insert(task.id.clone(), task.dependencies.clone());
            in_degree.insert(task.id.clone(), task.dependencies.len());
        }

        // Topological sort
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        // Find tasks with no dependencies
        for (task_id, degree) in &in_degree {
            if *degree == 0 {
                queue.push_back(task_id.clone());
            }
        }

        while let Some(task_id) = queue.pop_front() {
            result.push(task_id.clone());

            // Update dependencies
            for (dependent_id, dependencies) in &graph {
                if dependencies.contains(&task_id) {
                    let current_degree = in_degree.get_mut(dependent_id).ok_or_else(|| {
                        OrderingError::TopologicalSortFailed {
                            reason: format!("Task '{}' not found in in-degree map", dependent_id),
                        }
                    })?;
                    *current_degree -= 1;
                    if *current_degree == 0 {
                        queue.push_back(dependent_id.clone());
                    }
                }
            }
        }

        // Check for cycles
        if result.len() != plan.tasks.len() {
            return Err(OrderingError::TopologicalSortFailed {
                reason: "Circular dependency detected".to_string(),
            });
        }

        Ok(result)
    }

    fn detect_format(&self, content: &str) -> Result<PlanFormat, ParseError> {
        let trimmed = content.trim();

        if trimmed.starts_with('{') {
            Ok(PlanFormat::Json)
        } else if trimmed.starts_with("---") || trimmed.contains("metadata:") {
            Ok(PlanFormat::Yaml)
        } else {
            // Try to parse as JSON first, then YAML
            if serde_json::from_str::<serde_json::Value>(content).is_ok() {
                Ok(PlanFormat::Json)
            } else if serde_yaml::from_str::<serde_yaml::Value>(content).is_ok() {
                Ok(PlanFormat::Yaml)
            } else {
                Err(ParseError::UnknownFormat)
            }
        }
    }

    fn check_circular_dependencies(
        &self,
        tasks: &[crate::execution::Task],
    ) -> Result<(), DependencyError> {
        use std::collections::{HashMap, HashSet};

        let mut graph = HashMap::new();
        for task in tasks {
            graph.insert(task.id.clone(), task.dependencies.clone());
        }

        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for task in tasks {
            if !visited.contains(&task.id)
                && Self::has_cycle(&graph, &task.id, &mut visited, &mut rec_stack)
            {
                let cycle = self.find_cycle(&graph);
                return Err(DependencyError::CircularDependencies { cycle });
            }
        }

        Ok(())
    }

    fn has_cycle(
        graph: &HashMap<String, Vec<String>>,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if Self::has_cycle(graph, neighbor, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(neighbor) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }

    fn find_cycle(&self, _graph: &HashMap<String, Vec<String>>) -> Vec<String> {
        // Simplified cycle detection - in practice, we'd track the actual cycle
        vec!["cycle-detected".to_string()]
    }
}

impl Default for ExecutionPlanParser {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SchemaValidator {
    _json_schema: serde_json::Value,
}

impl Default for SchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaValidator {
    pub fn new() -> Self {
        // TODO: Load actual schema from embedded resource or file
        Self {
            _json_schema: serde_json::Value::Null,
        }
    }

    pub fn validate_plan(&self, plan: &ExecutionPlan) -> Result<(), ValidationError> {
        // Validate metadata version
        if plan.metadata.version != "1.0" {
            return Err(ValidationError::SchemaViolation {
                field: "metadata.version".to_string(),
                reason: format!("Unsupported version: {}", plan.metadata.version),
            });
        }

        // Validate plan has tasks
        if plan.tasks.is_empty() {
            return Err(ValidationError::SchemaViolation {
                field: "tasks".to_string(),
                reason: "Plan must contain at least one task".to_string(),
            });
        }

        // Validate each task
        for task in &plan.tasks {
            self.validate_task(task)?;
        }

        // Validate inventory
        self.validate_inventory(&plan.inventory)?;

        Ok(())
    }

    pub fn validate_task(&self, task: &crate::execution::Task) -> Result<(), ValidationError> {
        // Validate task ID
        if task.id.trim().is_empty() {
            return Err(ValidationError::SchemaViolation {
                field: "task.id".to_string(),
                reason: "Task ID cannot be empty".to_string(),
            });
        }

        // Validate module name
        if task.module.trim().is_empty() {
            return Err(ValidationError::SchemaViolation {
                field: "task.module".to_string(),
                reason: "Task module cannot be empty".to_string(),
            });
        }

        // Tasks can have empty args, so no validation needed for args presence

        Ok(())
    }

    pub fn validate_inventory(
        &self,
        inventory: &crate::execution::InventorySpec,
    ) -> Result<(), ValidationError> {
        // Validate inventory based on type
        match &inventory.source {
            crate::execution::InventorySource::Inline { content } => {
                // Validate inline inventory content is not empty
                if content.trim().is_empty() {
                    return Err(ValidationError::SchemaViolation {
                        field: "inventory.content".to_string(),
                        reason: "Inline inventory content cannot be empty".to_string(),
                    });
                }
            }
            crate::execution::InventorySource::File { path } => {
                // Validate file path is not empty
                if path.trim().is_empty() {
                    return Err(ValidationError::SchemaViolation {
                        field: "inventory.file".to_string(),
                        reason: "Inventory file path cannot be empty".to_string(),
                    });
                }
            }
            crate::execution::InventorySource::Dynamic { script } => {
                // Validate dynamic script path
                if script.trim().is_empty() {
                    return Err(ValidationError::SchemaViolation {
                        field: "inventory.dynamic".to_string(),
                        reason: "Dynamic inventory script path cannot be empty".to_string(),
                    });
                }
            }
            crate::execution::InventorySource::Url { url } => {
                // Validate URL is not empty
                if url.trim().is_empty() {
                    return Err(ValidationError::SchemaViolation {
                        field: "inventory.url".to_string(),
                        reason: "Inventory URL cannot be empty".to_string(),
                    });
                }
            }
        }

        Ok(())
    }
}

pub struct TemplateProcessor {
    _engine: TemplateEngine,
}

impl Default for TemplateProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateProcessor {
    pub fn new() -> Self {
        Self {
            _engine: TemplateEngine::new(),
        }
    }

    pub fn process_plan(
        &self,
        plan: &ExecutionPlan,
        _variables: &HashMap<String, serde_json::Value>,
    ) -> Result<ExecutionPlan, TemplateError> {
        // TODO: Implement template processing using handlebars
        // For now, return the plan unchanged
        Ok(plan.clone())
    }

    pub fn process_task_args(
        &self,
        args: &HashMap<String, serde_json::Value>,
        _variables: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>, TemplateError> {
        // TODO: Implement argument template processing
        Ok(args.clone())
    }
}

struct TemplateEngine {
    _handlebars: handlebars::Handlebars<'static>,
}

impl TemplateEngine {
    fn new() -> Self {
        Self {
            _handlebars: handlebars::Handlebars::new(),
        }
    }
}
