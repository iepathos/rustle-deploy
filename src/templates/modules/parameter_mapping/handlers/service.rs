use super::super::{ModuleParameterHandler, ParameterError};
use serde_json::Value;
use std::collections::HashMap;

pub struct ServiceParameterHandler;

impl ModuleParameterHandler for ServiceParameterHandler {
    fn map_parameters(
        &self,
        mut ansible_params: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, ParameterError> {
        let mut mapped = HashMap::new();

        // Handle name parameter (required)
        if let Some(name) = ansible_params.remove("name") {
            mapped.insert("name".to_string(), name);
        }

        // Handle state parameter (default to started if not specified)
        let state = ansible_params
            .remove("state")
            .unwrap_or_else(|| Value::String("started".to_string()));
        mapped.insert("state".to_string(), state);

        // Handle enabled parameter (default to no change if not specified)
        if let Some(enabled) = ansible_params.remove("enabled") {
            mapped.insert("enabled".to_string(), enabled);
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
        HashMap::new() // No aliases for service module currently
    }

    fn validate_parameters(&self, params: &HashMap<String, Value>) -> Result<(), ParameterError> {
        if !params.contains_key("name") {
            return Err(ParameterError::MissingRequired {
                param: "name".to_string(),
            });
        }

        // Validate state parameter if present
        if let Some(state) = params.get("state") {
            if let Some(state_str) = state.as_str() {
                match state_str {
                    "started" | "stopped" | "restarted" | "reloaded" => {}
                    _ => {
                        return Err(ParameterError::InvalidValue {
                            param: "state".to_string(),
                            reason: "must be one of: started, stopped, restarted, reloaded"
                                .to_string(),
                        });
                    }
                }
            }
        }

        Ok(())
    }
}
