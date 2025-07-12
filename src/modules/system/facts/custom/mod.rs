//! Custom facts loader

use crate::modules::system::facts::FactError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::process::Command;

pub struct CustomFactsLoader {
    fact_paths: Vec<PathBuf>,
}

impl CustomFactsLoader {
    pub fn new(fact_paths: Vec<PathBuf>) -> Self {
        Self { fact_paths }
    }

    pub async fn load_custom_facts(&self) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut custom_facts = HashMap::new();

        for path in &self.fact_paths {
            if path.is_dir() {
                custom_facts.extend(self.load_fact_directory(path).await?);
            } else if path.is_file() {
                custom_facts.extend(self.load_fact_file(path).await?);
            }
        }

        Ok(custom_facts)
    }

    async fn load_fact_directory(
        &self,
        dir_path: &Path,
    ) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();

        let mut entries =
            fs::read_dir(dir_path)
                .await
                .map_err(|_e| FactError::CustomFactError {
                    path: dir_path.to_string_lossy().to_string(),
                })?;

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_file() {
                if let Ok(file_facts) = self.load_fact_file(&path).await {
                    facts.extend(file_facts);
                }
            }
        }

        Ok(facts)
    }

    async fn load_fact_file(
        &self,
        path: &Path,
    ) -> Result<HashMap<String, serde_json::Value>, FactError> {
        match path.extension().and_then(|s| s.to_str()) {
            Some("json") => {
                let content = fs::read_to_string(path).await?;
                let facts: HashMap<String, serde_json::Value> = serde_json::from_str(&content)?;
                Ok(facts)
            }
            Some("yaml") | Some("yml") => {
                let content = fs::read_to_string(path).await?;
                let facts: HashMap<String, serde_json::Value> = serde_yaml::from_str(&content)?;
                Ok(facts)
            }
            _ => {
                // Execute as script and capture JSON output
                self.execute_fact_script(path).await
            }
        }
    }

    async fn execute_fact_script(
        &self,
        script_path: &Path,
    ) -> Result<HashMap<String, serde_json::Value>, FactError> {
        // Make script executable if it isn't already
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = fs::metadata(script_path).await {
                let permissions = metadata.permissions();
                if permissions.mode() & 0o111 == 0 {
                    // Script is not executable, try to make it executable
                    let mut new_permissions = permissions.clone();
                    new_permissions.set_mode(permissions.mode() | 0o755);
                    if (fs::set_permissions(script_path, new_permissions).await).is_err() {
                        // If we can't make it executable, we can't run it
                        return Ok(HashMap::new());
                    }
                }
            }
        }

        // Execute the script
        let output =
            Command::new(script_path)
                .output()
                .await
                .map_err(|_| FactError::CustomFactError {
                    path: script_path.to_string_lossy().to_string(),
                })?;

        if !output.status.success() {
            return Ok(HashMap::new());
        }

        // Try to parse output as JSON
        match serde_json::from_slice::<HashMap<String, serde_json::Value>>(&output.stdout) {
            Ok(facts) => Ok(facts),
            Err(_) => {
                // If not valid JSON, treat output as a single string fact
                let fact_name = script_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("custom_fact");

                let mut facts = HashMap::new();
                facts.insert(
                    fact_name.to_string(),
                    serde_json::Value::String(
                        String::from_utf8_lossy(&output.stdout).trim().to_string(),
                    ),
                );
                Ok(facts)
            }
        }
    }
}
