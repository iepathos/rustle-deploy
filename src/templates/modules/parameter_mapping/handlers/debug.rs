use super::super::{ModuleParameterHandler, ParameterError};
use serde_json::Value;
use std::collections::HashMap;

pub struct DebugParameterHandler;

impl ModuleParameterHandler for DebugParameterHandler {
    fn map_parameters(
        &self,
        mut ansible_params: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, ParameterError> {
        let mut mapped = HashMap::new();

        // Handle msg parameter
        if let Some(msg) = ansible_params.remove("msg") {
            mapped.insert("msg".to_string(), msg);
        }

        // Handle var parameter
        if let Some(var) = ansible_params.remove("var") {
            mapped.insert("var".to_string(), var);
        }

        // Handle verbosity parameter (debug output level)
        if let Some(verbosity) = ansible_params.remove("verbosity") {
            mapped.insert("verbosity".to_string(), verbosity);
        }

        // Pass through other parameters
        for (key, value) in ansible_params {
            mapped.insert(key, value);
        }

        Ok(mapped)
    }

    fn required_parameters(&self) -> Vec<&'static str> {
        // Debug module requires either msg or var
        vec![]
    }

    fn parameter_aliases(&self) -> HashMap<&'static str, Vec<&'static str>> {
        HashMap::new() // No aliases for debug module currently
    }

    fn validate_parameters(&self, params: &HashMap<String, Value>) -> Result<(), ParameterError> {
        // Debug module requires either msg or var parameter
        if !params.contains_key("msg") && !params.contains_key("var") {
            return Err(ParameterError::MissingRequired {
                param: "msg or var".to_string(),
            });
        }

        // If both are provided, that's conflicting
        if params.contains_key("msg") && params.contains_key("var") {
            return Err(ParameterError::ConflictingParameters {
                params: vec!["msg".to_string(), "var".to_string()],
            });
        }

        Ok(())
    }
}
