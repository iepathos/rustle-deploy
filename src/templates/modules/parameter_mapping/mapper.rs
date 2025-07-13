use serde_json::Value;
use std::collections::HashMap;
use tracing::debug;

use super::{
    handlers::{
        CommandParameterHandler, CopyParameterHandler, DebugParameterHandler, FileParameterHandler,
        PackageParameterHandler, ServiceParameterHandler,
    },
    ModuleParameterHandler, ParameterError,
};

pub struct ParameterMapper {
    module_handlers: HashMap<String, Box<dyn ModuleParameterHandler>>,
}

impl ParameterMapper {
    pub fn new() -> Self {
        let mut handlers: HashMap<String, Box<dyn ModuleParameterHandler>> = HashMap::new();

        handlers.insert("command".to_string(), Box::new(CommandParameterHandler));
        handlers.insert("shell".to_string(), Box::new(CommandParameterHandler));
        handlers.insert("copy".to_string(), Box::new(CopyParameterHandler));
        handlers.insert("file".to_string(), Box::new(FileParameterHandler));
        handlers.insert("package".to_string(), Box::new(PackageParameterHandler));
        handlers.insert("service".to_string(), Box::new(ServiceParameterHandler));
        handlers.insert("debug".to_string(), Box::new(DebugParameterHandler));

        // Package management modules - all use PackageParameterHandler
        handlers.insert("apt".to_string(), Box::new(PackageParameterHandler));
        handlers.insert("yum".to_string(), Box::new(PackageParameterHandler));
        handlers.insert("dnf".to_string(), Box::new(PackageParameterHandler));
        handlers.insert("zypper".to_string(), Box::new(PackageParameterHandler));

        // Service management modules - all use ServiceParameterHandler
        handlers.insert("systemd".to_string(), Box::new(ServiceParameterHandler));

        Self {
            module_handlers: handlers,
        }
    }

    pub fn map_for_module(
        &self,
        module_name: &str,
        params: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, ParameterError> {
        debug!(
            "Mapping parameters for module '{}': {:?}",
            module_name, params
        );

        let handler = self.module_handlers.get(module_name).ok_or_else(|| {
            ParameterError::UnknownParameter {
                param: format!("module: {module_name}"),
            }
        })?;

        let mapped = handler.map_parameters(params)?;
        handler.validate_parameters(&mapped)?;

        debug!("Mapped parameters: {:?}", mapped);
        Ok(mapped)
    }
}

impl Default for ParameterMapper {
    fn default() -> Self {
        Self::new()
    }
}
