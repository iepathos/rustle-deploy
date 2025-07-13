use super::super::error::ParameterError;
use super::super::ModuleParameterHandler;
use serde_json::Value;
use std::collections::HashMap;

pub struct WaitForHandler;

impl ModuleParameterHandler for WaitForHandler {
    fn map_parameters(
        &self,
        ansible_params: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, ParameterError> {
        let mut mapped = HashMap::new();

        // Map host parameter
        if let Some(host) = ansible_params.get("host") {
            mapped.insert("host".to_string(), host.clone());
        }

        // Map port parameter (required)
        if let Some(port) = ansible_params.get("port") {
            mapped.insert("port".to_string(), port.clone());
        }

        // Map timeout parameter
        if let Some(timeout) = ansible_params.get("timeout") {
            mapped.insert("timeout".to_string(), timeout.clone());
        }

        // Map delay parameter
        if let Some(delay) = ansible_params.get("delay") {
            mapped.insert("delay".to_string(), delay.clone());
        }

        // Validate required parameters
        self.validate_parameters(&mapped)?;

        Ok(mapped)
    }

    fn required_parameters(&self) -> Vec<&'static str> {
        vec!["port"]
    }

    fn parameter_aliases(&self) -> HashMap<&'static str, Vec<&'static str>> {
        HashMap::new()
    }

    fn validate_parameters(&self, params: &HashMap<String, Value>) -> Result<(), ParameterError> {
        if !params.contains_key("port") {
            return Err(ParameterError::MissingRequired {
                param: "port".to_string(),
            });
        }

        // Validate port is a number
        if let Some(port) = params.get("port") {
            if !port.is_number() {
                return Err(ParameterError::InvalidValue {
                    param: "port".to_string(),
                    reason: "Expected a number".to_string(),
                });
            }
        }

        Ok(())
    }
}
