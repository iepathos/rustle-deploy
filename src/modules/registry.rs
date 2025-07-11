//! Central registry for all execution modules

use crate::modules::{
    error::ModuleExecutionError,
    interface::{ExecutionContext, ExecutionModule, ModuleArgs, ModuleResult},
};
use std::collections::HashMap;

/// Central registry for all execution modules
pub struct ModuleRegistry {
    modules: HashMap<String, Box<dyn ExecutionModule>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    /// Create a registry with all core modules pre-registered
    pub fn with_core_modules() -> Self {
        let mut registry = Self::new();

        // Register core modules
        registry.register(Box::new(crate::modules::core::DebugModule));
        registry.register(Box::new(crate::modules::core::CommandModule));
        registry.register(Box::new(crate::modules::core::PackageModule::new()));
        registry.register(Box::new(crate::modules::core::ServiceModule::new()));

        registry
    }

    pub fn register(&mut self, module: Box<dyn ExecutionModule>) {
        self.modules.insert(module.name().to_string(), module);
    }

    pub fn get_module(&self, name: &str) -> Option<&dyn ExecutionModule> {
        self.modules.get(name).map(|m| m.as_ref())
    }

    pub fn list_modules(&self) -> Vec<&str> {
        self.modules.keys().map(|s| s.as_str()).collect()
    }

    pub async fn execute_module(
        &self,
        module_name: &str,
        args: &ModuleArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        let module = self
            .get_module(module_name)
            .ok_or_else(|| ModuleExecutionError::ModuleNotFound(module_name.to_string()))?;

        module.validate_args(args)?;

        if context.check_mode {
            module.check_mode(args, context).await
        } else {
            module.execute(args, context).await
        }
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}
