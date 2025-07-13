use crate::execution::plan_converter::RustlePlanConverter;
use crate::execution::rustle_plan::{BinaryDeploymentPlan, RustlePlanOutput, StaticFileRef};
use crate::types::deployment::RuntimeConfig;
use anyhow::Result;
use std::collections::HashMap;
use thiserror::Error;

use super::{EmbeddedData, EncryptedSecrets, TargetInfo, TemplateConfig};

#[derive(Error, Debug)]
pub enum EmbedError {
    #[error("Serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Plan conversion failed: {0}")]
    PlanConversion(#[from] crate::execution::compatibility::ConversionError),
}

pub struct DataEmbedder {
    _config: TemplateConfig,
}

impl DataEmbedder {
    pub fn new(config: &TemplateConfig) -> Result<Self> {
        Ok(Self {
            _config: config.clone(),
        })
    }

    pub async fn embed_execution_data(
        &self,
        execution_plan: &RustlePlanOutput,
        binary_deployment: &BinaryDeploymentPlan,
        _target_info: &TargetInfo,
    ) -> Result<EmbeddedData, EmbedError> {
        // Convert RustlePlanOutput to ExecutionPlan to properly handle condition conversion
        let converter = RustlePlanConverter::new();
        let converted_plan = converter.convert_to_execution_plan(execution_plan)?;
        let execution_plan_json = serde_json::to_string_pretty(&converted_plan)?;

        let runtime_config = RuntimeConfig {
            controller_endpoint: binary_deployment.controller_endpoint.clone(),
            execution_timeout: binary_deployment
                .execution_timeout
                .unwrap_or(std::time::Duration::from_secs(300)),
            report_interval: binary_deployment
                .report_interval
                .unwrap_or(std::time::Duration::from_secs(30)),
            cleanup_on_completion: binary_deployment.cleanup_on_completion.unwrap_or(true),
            log_level: binary_deployment
                .log_level
                .clone()
                .unwrap_or_else(|| String::from("info")),
            verbose: binary_deployment.verbose.unwrap_or(false),
        };

        let secrets = EncryptedSecrets {
            vault_data: HashMap::new(),
            encryption_key_id: String::from("none"),
            decryption_method: String::from("none"),
        };

        // Process static files from binary deployment, handling missing files gracefully
        let mut static_files = HashMap::new();
        for static_file_ref in &binary_deployment.static_files {
            match self.load_static_file(static_file_ref).await {
                Ok((path, content)) => {
                    static_files.insert(path, content);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to load static file '{}': {}. Skipping file for stdin compatibility.",
                        static_file_ref.source_path,
                        e
                    );
                    // Continue without the file - this allows stdin input to work
                    // even when static file paths can't be resolved
                }
            }
        }

        Ok(EmbeddedData {
            execution_plan: execution_plan_json,
            static_files,
            module_binaries: HashMap::new(),
            runtime_config,
            secrets,
            facts_cache: None,
        })
    }

    /// Load a static file, returning the target path and file contents
    async fn load_static_file(
        &self,
        static_file_ref: &StaticFileRef,
    ) -> Result<(String, Vec<u8>), EmbedError> {
        let content = tokio::fs::read(&static_file_ref.source_path).await?;
        Ok((static_file_ref.target_path.clone(), content))
    }
}
