use serde_json::Value;
use std::collections::HashMap;

pub mod error;
pub mod handlers;
pub mod mapper;

pub use error::ParameterError;
pub use mapper::ParameterMapper;

pub trait ModuleParameterHandler {
    /// Map Ansible-style parameters to module-expected parameters
    fn map_parameters(
        &self,
        ansible_params: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, ParameterError>;

    /// Get required parameters for this module
    fn required_parameters(&self) -> Vec<&'static str>;

    /// Get parameter aliases for this module
    fn parameter_aliases(&self) -> HashMap<&'static str, Vec<&'static str>>;

    /// Validate that all required parameters are present
    fn validate_parameters(&self, params: &HashMap<String, Value>) -> Result<(), ParameterError>;
}
