use super::super::{ModuleParameterHandler, ParameterError};
use serde_json::Value;
use std::collections::HashMap;

pub struct CommandParameterHandler;

impl ModuleParameterHandler for CommandParameterHandler {
    fn map_parameters(
        &self,
        mut ansible_params: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, ParameterError> {
        let mut mapped = HashMap::new();

        // Handle _raw_params -> cmd mapping (highest priority for Ansible compatibility)
        if let Some(raw_params) = ansible_params.remove("_raw_params") {
            mapped.insert("cmd".to_string(), raw_params);
        }
        // Handle existing cmd/command parameters (lower priority)
        else if let Some(cmd) = ansible_params.remove("cmd") {
            mapped.insert("cmd".to_string(), cmd);
        } else if let Some(command) = ansible_params.remove("command") {
            mapped.insert("cmd".to_string(), command);
        }

        // Handle chdir parameter
        if let Some(chdir) = ansible_params.remove("chdir") {
            mapped.insert("chdir".to_string(), chdir);
        }

        // Handle creates/removes parameters
        if let Some(creates) = ansible_params.remove("creates") {
            mapped.insert("creates".to_string(), creates);
        }
        if let Some(removes) = ansible_params.remove("removes") {
            mapped.insert("removes".to_string(), removes);
        }

        // Pass through any other parameters
        for (key, value) in ansible_params {
            mapped.insert(key, value);
        }

        Ok(mapped)
    }

    fn required_parameters(&self) -> Vec<&'static str> {
        vec!["cmd"]
    }

    fn parameter_aliases(&self) -> HashMap<&'static str, Vec<&'static str>> {
        let mut aliases = HashMap::new();
        aliases.insert("cmd", vec!["command", "_raw_params"]);
        aliases
    }

    fn validate_parameters(&self, params: &HashMap<String, Value>) -> Result<(), ParameterError> {
        if !params.contains_key("cmd") {
            return Err(ParameterError::MissingRequired {
                param: "cmd (or _raw_params, command)".to_string(),
            });
        }
        Ok(())
    }
}
