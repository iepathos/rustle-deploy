use super::super::{ModuleParameterHandler, ParameterError};
use serde_json::Value;
use std::collections::HashMap;

pub struct FileParameterHandler;

impl ModuleParameterHandler for FileParameterHandler {
    fn map_parameters(
        &self,
        mut ansible_params: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, ParameterError> {
        let mut mapped = HashMap::new();

        // Handle path parameter (required)
        if let Some(path) = ansible_params.remove("path") {
            mapped.insert("path".to_string(), path);
        }

        // Handle state parameter
        if let Some(state) = ansible_params.remove("state") {
            mapped.insert("state".to_string(), state);
        }

        // Handle mode parameter (file permissions)
        if let Some(mode) = ansible_params.remove("mode") {
            mapped.insert("mode".to_string(), mode);
        }

        // Handle owner parameter
        if let Some(owner) = ansible_params.remove("owner") {
            mapped.insert("owner".to_string(), owner);
        }

        // Handle group parameter
        if let Some(group) = ansible_params.remove("group") {
            mapped.insert("group".to_string(), group);
        }

        // Handle src parameter (for link operations)
        if let Some(src) = ansible_params.remove("src") {
            mapped.insert("src".to_string(), src);
        }

        // Handle dest parameter - for file module, dest is often the path for link operations
        if let Some(dest) = ansible_params.remove("dest") {
            // If we don't have a path yet, use dest as path
            if !mapped.contains_key("path") {
                mapped.insert("path".to_string(), dest.clone());
            }
            mapped.insert("dest".to_string(), dest);
        }

        // Handle recurse parameter
        if let Some(recurse) = ansible_params.remove("recurse") {
            mapped.insert("recurse".to_string(), recurse);
        }

        // Handle follow parameter
        if let Some(follow) = ansible_params.remove("follow") {
            mapped.insert("follow".to_string(), follow);
        }

        // Handle force parameter
        if let Some(force) = ansible_params.remove("force") {
            mapped.insert("force".to_string(), force);
        }

        // Handle backup parameter
        if let Some(backup) = ansible_params.remove("backup") {
            mapped.insert("backup".to_string(), backup);
        }

        // Pass through other parameters
        for (key, value) in ansible_params {
            mapped.insert(key, value);
        }

        Ok(mapped)
    }

    fn required_parameters(&self) -> Vec<&'static str> {
        vec!["path"]
    }

    fn parameter_aliases(&self) -> HashMap<&'static str, Vec<&'static str>> {
        let mut aliases = HashMap::new();
        aliases.insert("src", vec!["dest"]); // dest can be alias for src in link operations
        aliases
    }

    fn validate_parameters(&self, params: &HashMap<String, Value>) -> Result<(), ParameterError> {
        // Path parameter is required (should be mapped from dest if not present)
        if !params.contains_key("path") {
            return Err(ParameterError::MissingRequired {
                param: "path".to_string(),
            });
        }

        // If state is link, src parameter is required
        if let Some(Value::String(state)) = params.get("state") {
            if state == "link" && !params.contains_key("src") {
                return Err(ParameterError::MissingRequired {
                    param: "src (required for link state)".to_string(),
                });
            }
        }

        Ok(())
    }
}
