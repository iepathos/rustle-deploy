use super::super::{ModuleParameterHandler, ParameterError};
use serde_json::Value;
use std::collections::HashMap;

pub struct PackageParameterHandler;

impl ModuleParameterHandler for PackageParameterHandler {
    fn map_parameters(
        &self,
        mut ansible_params: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, ParameterError> {
        let mut mapped = HashMap::new();

        // Handle name/pkg parameter aliases
        if let Some(name) = ansible_params.remove("name") {
            mapped.insert("name".to_string(), name);
        } else if let Some(pkg) = ansible_params.remove("pkg") {
            mapped.insert("name".to_string(), pkg);
        }

        // Handle state parameter (default to present)
        let state = ansible_params
            .remove("state")
            .unwrap_or_else(|| Value::String("present".to_string()));
        mapped.insert("state".to_string(), state);

        // Handle version parameter
        if let Some(version) = ansible_params.remove("version") {
            mapped.insert("version".to_string(), version);
        }

        // Pass through other parameters
        for (key, value) in ansible_params {
            mapped.insert(key, value);
        }

        Ok(mapped)
    }

    fn required_parameters(&self) -> Vec<&'static str> {
        vec!["name"]
    }

    fn parameter_aliases(&self) -> HashMap<&'static str, Vec<&'static str>> {
        let mut aliases = HashMap::new();
        aliases.insert("name", vec!["pkg"]);
        aliases
    }

    fn validate_parameters(&self, params: &HashMap<String, Value>) -> Result<(), ParameterError> {
        if !params.contains_key("name") {
            return Err(ParameterError::MissingRequired {
                param: "name (or pkg)".to_string(),
            });
        }
        Ok(())
    }
}
