use crate::deploy::{DeployError, Result};
use crate::execution::ExecutionPlan;
use crate::runtime::RuntimeConfig;
use handlebars::Handlebars;
use serde_json::json;
use std::collections::HashMap;

/// Generator for runtime binary templates
pub struct RuntimeTemplateGenerator {
    handlebars: Handlebars<'static>,
}

impl RuntimeTemplateGenerator {
    pub fn new() -> Result<Self> {
        let mut handlebars = Handlebars::new();

        // Register built-in templates
        handlebars
            .register_template_string(
                "runtime_main",
                include_str!("../../templates/runtime_main.rs.template"),
            )
            .map_err(|e| {
                DeployError::TemplateGeneration(format!("Failed to register main template: {e}"))
            })?;

        handlebars
            .register_template_string(
                "runtime_cargo",
                include_str!("../../templates/runtime_cargo.toml.template"),
            )
            .map_err(|e| {
                DeployError::TemplateGeneration(format!("Failed to register cargo template: {e}"))
            })?;

        Ok(Self { handlebars })
    }

    /// Generate main.rs for the runtime binary
    pub fn generate_main_rs(
        &self,
        execution_plan: &ExecutionPlan,
        runtime_config: &RuntimeConfig,
    ) -> Result<String> {
        self.generate_main_rs_with_modules(execution_plan, runtime_config, &[])
    }

    /// Generate main.rs for the runtime binary with custom compiled modules
    pub fn generate_main_rs_with_modules(
        &self,
        execution_plan: &ExecutionPlan,
        runtime_config: &RuntimeConfig,
        compiled_modules: &[crate::modules::CompiledModule],
    ) -> Result<String> {
        let execution_plan_json = serde_json::to_string(execution_plan).map_err(|e| {
            DeployError::TemplateGeneration(format!("Failed to serialize execution plan: {e}"))
        })?;

        let runtime_config_json = serde_json::to_string(runtime_config).map_err(|e| {
            DeployError::TemplateGeneration(format!("Failed to serialize runtime config: {e}"))
        })?;

        // Include the runtime execution engine code
        let runtime_code = self.generate_runtime_code()?;

        // Generate module registry code
        let module_registry_code = self.generate_module_registry_code(compiled_modules)?;

        let template_data = json!({
            "execution_plan": serde_json::to_string(&execution_plan_json)?,
            "runtime_config": serde_json::to_string(&runtime_config_json)?,
            "runtime_code": runtime_code,
            "module_registry_code": module_registry_code,
            "has_custom_modules": !compiled_modules.is_empty()
        });

        self.handlebars
            .render("runtime_main", &template_data)
            .map_err(|e| {
                DeployError::TemplateGeneration(format!("Failed to render main template: {e}"))
            })
    }

    /// Generate Cargo.toml for the runtime binary
    pub fn generate_cargo_toml(&self, binary_name: &str) -> Result<String> {
        let template_data = json!({
            "binary_name": binary_name
        });

        self.handlebars
            .render("runtime_cargo", &template_data)
            .map_err(|e| {
                DeployError::TemplateGeneration(format!("Failed to render cargo template: {e}"))
            })
    }

    /// Generate the complete runtime execution engine code
    fn generate_runtime_code(&self) -> Result<String> {
        // Include all the runtime module source code inline
        let mut runtime_code = String::new();

        // Error types
        runtime_code.push_str(include_str!("../runtime/error.rs"));
        runtime_code.push('\n');

        // State management
        runtime_code.push_str(include_str!("../runtime/state.rs"));
        runtime_code.push('\n');

        // Progress reporting
        runtime_code.push_str(include_str!("../runtime/progress.rs"));
        runtime_code.push('\n');

        // Facts collection
        runtime_code.push_str(include_str!("../runtime/facts.rs"));
        runtime_code.push('\n');

        // Condition evaluation
        runtime_code.push_str(include_str!("../runtime/conditions.rs"));
        runtime_code.push('\n');

        // Main executor
        runtime_code.push_str(include_str!("../runtime/executor.rs"));
        runtime_code.push('\n');

        // Module interface and registry
        runtime_code.push_str(include_str!("../modules/interface.rs"));
        runtime_code.push('\n');
        runtime_code.push_str(include_str!("../modules/registry.rs"));
        runtime_code.push('\n');
        runtime_code.push_str(include_str!("../modules/error.rs"));
        runtime_code.push('\n');

        // Core modules
        runtime_code.push_str(include_str!("../modules/core/debug.rs"));
        runtime_code.push('\n');
        runtime_code.push_str(include_str!("../modules/core/command.rs"));
        runtime_code.push('\n');
        runtime_code.push_str(include_str!("../modules/core/package.rs"));
        runtime_code.push('\n');
        runtime_code.push_str(include_str!("../modules/core/service.rs"));
        runtime_code.push('\n');

        // Execution plan types
        runtime_code.push_str(include_str!("../execution/plan.rs"));
        runtime_code.push('\n');

        Ok(runtime_code)
    }

    /// Generate module registry code for compiled modules
    fn generate_module_registry_code(
        &self,
        compiled_modules: &[crate::modules::CompiledModule],
    ) -> Result<String> {
        if compiled_modules.is_empty() {
            return Ok(String::new());
        }

        let mut code = String::new();

        code.push_str("// Auto-generated compiled modules\n\n");

        // Include each compiled module
        for (i, module) in compiled_modules.iter().enumerate() {
            code.push_str(&format!("// Module: {}\n", module.spec.name));
            code.push_str(&format!("mod compiled_module_{i};\n"));
            code.push_str(&module.compiled_code);
            code.push_str("\n\n");
        }

        // Generate registration function
        code.push_str("/// Register all compiled modules\n");
        code.push_str("fn register_compiled_modules(registry: &mut crate::modules::registry::ModuleRegistry) {\n");

        for module in compiled_modules {
            code.push_str(&format!("    // Register {}\n", module.spec.name));
            code.push_str(&format!("    {};\n", module.registration_code));
        }

        code.push_str("}\n");

        Ok(code)
    }

    /// Generate lib.rs for the runtime binary (if needed as a library)
    pub fn generate_lib_rs(&self) -> Result<String> {
        Ok(r#"
//! Runtime execution engine for embedded binaries

pub mod runtime;
pub mod modules;
pub mod execution;

pub use runtime::*;
pub use modules::*;
pub use execution::*;
"#
        .to_string())
    }

    /// Generate a complete binary project structure
    pub fn generate_binary_project(
        &self,
        binary_name: &str,
        execution_plan: &ExecutionPlan,
        runtime_config: &RuntimeConfig,
    ) -> Result<HashMap<String, String>> {
        let mut files = HashMap::new();

        // Generate main.rs
        files.insert(
            "src/main.rs".to_string(),
            self.generate_main_rs(execution_plan, runtime_config)?,
        );

        // Generate Cargo.toml
        files.insert(
            "Cargo.toml".to_string(),
            self.generate_cargo_toml(binary_name)?,
        );

        // Generate lib.rs (optional)
        files.insert("src/lib.rs".to_string(), self.generate_lib_rs()?);

        Ok(files)
    }
}

impl Default for RuntimeTemplateGenerator {
    fn default() -> Self {
        Self::new().expect("Failed to create RuntimeTemplateGenerator")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::{ExecutionPlanMetadata, FailurePolicy, TargetSelector, Task, TaskType};
    use chrono::Utc;
    use std::collections::HashMap;

    #[test]
    fn test_generate_main_rs() {
        let generator = RuntimeTemplateGenerator::new().unwrap();

        let execution_plan = ExecutionPlan {
            metadata: ExecutionPlanMetadata {
                version: "1.0.0".to_string(),
                created_at: Utc::now(),
                rustle_plan_version: "1.0.0".to_string(),
                plan_id: "test-plan".to_string(),
                description: Some("Test plan".to_string()),
                author: Some("test".to_string()),
                tags: vec![],
            },
            tasks: vec![Task {
                id: "test-task".to_string(),
                name: "Test Task".to_string(),
                task_type: TaskType::Command,
                module: "debug".to_string(),
                args: [("msg".to_string(), serde_json::json!("Hello, World!"))].into(),
                dependencies: vec![],
                conditions: vec![],
                target_hosts: TargetSelector::All,
                timeout: None,
                retry_policy: None,
                failure_policy: FailurePolicy::Abort,
            }],
            inventory: crate::execution::InventorySpec {
                format: crate::execution::InventoryFormat::Json,
                source: crate::execution::InventorySource::Inline {
                    content: "{}".to_string(),
                },
                groups: HashMap::new(),
                hosts: HashMap::new(),
                variables: HashMap::new(),
            },
            strategy: crate::execution::ExecutionStrategy::Linear,
            facts_template: crate::execution::FactsTemplate {
                global_facts: vec![],
                host_facts: vec![],
                custom_facts: HashMap::new(),
            },
            deployment_config: crate::execution::DeploymentConfig {
                target_path: "/tmp/test".to_string(),
                backup_previous: false,
                verify_deployment: false,
                cleanup_on_success: false,
                deployment_timeout: None,
            },
            modules: vec![],
        };

        let runtime_config = RuntimeConfig::default();

        let main_rs = generator
            .generate_main_rs(&execution_plan, &runtime_config)
            .unwrap();

        assert!(main_rs.contains("fn main()"));
        assert!(main_rs.contains("LocalExecutor"));
        assert!(main_rs.contains("execute_plan"));
    }

    #[test]
    fn test_generate_cargo_toml() {
        let generator = RuntimeTemplateGenerator::new().unwrap();

        let cargo_toml = generator.generate_cargo_toml("test-binary").unwrap();

        assert!(cargo_toml.contains("name = \"test-binary\""));
        assert!(cargo_toml.contains("tokio"));
        assert!(cargo_toml.contains("serde"));
    }
}
