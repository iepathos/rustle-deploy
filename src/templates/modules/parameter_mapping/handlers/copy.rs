use super::super::{ModuleParameterHandler, ParameterError};
use serde_json::Value;
use std::collections::HashMap;

pub struct CopyParameterHandler;

impl ModuleParameterHandler for CopyParameterHandler {
    fn map_parameters(
        &self,
        mut ansible_params: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, ParameterError> {
        let mut mapped = HashMap::new();

        // Handle src parameter (required)
        if let Some(src) = ansible_params.remove("src") {
            mapped.insert("src".to_string(), src);
        }

        // Handle dest parameter (required)
        if let Some(dest) = ansible_params.remove("dest") {
            mapped.insert("dest".to_string(), dest);
        }

        // Handle backup parameter
        if let Some(backup) = ansible_params.remove("backup") {
            mapped.insert("backup".to_string(), backup);
        }

        // Handle force parameter
        if let Some(force) = ansible_params.remove("force") {
            mapped.insert("force".to_string(), force);
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

        // Handle directory_mode parameter
        if let Some(directory_mode) = ansible_params.remove("directory_mode") {
            mapped.insert("directory_mode".to_string(), directory_mode);
        }

        // Handle validate parameter
        if let Some(validate) = ansible_params.remove("validate") {
            mapped.insert("validate".to_string(), validate);
        }

        // Handle checksum parameter
        if let Some(checksum) = ansible_params.remove("checksum") {
            mapped.insert("checksum".to_string(), checksum);
        }

        // Handle follow parameter
        if let Some(follow) = ansible_params.remove("follow") {
            mapped.insert("follow".to_string(), follow);
        }

        // Handle preserve parameter
        if let Some(preserve) = ansible_params.remove("preserve") {
            mapped.insert("preserve".to_string(), preserve);
        }

        // Handle remote_src parameter
        if let Some(remote_src) = ansible_params.remove("remote_src") {
            mapped.insert("remote_src".to_string(), remote_src);
        }

        // Handle local_follow parameter
        if let Some(local_follow) = ansible_params.remove("local_follow") {
            mapped.insert("local_follow".to_string(), local_follow);
        }

        // Handle decrypt parameter
        if let Some(decrypt) = ansible_params.remove("decrypt") {
            mapped.insert("decrypt".to_string(), decrypt);
        }

        // Pass through other parameters
        for (key, value) in ansible_params {
            mapped.insert(key, value);
        }

        Ok(mapped)
    }

    fn required_parameters(&self) -> Vec<&'static str> {
        vec!["src", "dest"]
    }

    fn parameter_aliases(&self) -> HashMap<&'static str, Vec<&'static str>> {
        HashMap::new() // No aliases for copy module currently
    }

    fn validate_parameters(&self, params: &HashMap<String, Value>) -> Result<(), ParameterError> {
        // src parameter is required
        if !params.contains_key("src") {
            return Err(ParameterError::MissingRequired {
                param: "src".to_string(),
            });
        }

        // dest parameter is required
        if !params.contains_key("dest") {
            return Err(ParameterError::MissingRequired {
                param: "dest".to_string(),
            });
        }

        Ok(())
    }
}
