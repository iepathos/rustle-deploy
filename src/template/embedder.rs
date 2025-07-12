use crate::execution::rustle_plan::{BinaryDeploymentPlan, RustlePlanOutput};
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
        let execution_plan_json = serde_json::to_string_pretty(execution_plan)?;

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

        Ok(EmbeddedData {
            execution_plan: execution_plan_json,
            static_files: HashMap::new(),
            module_binaries: HashMap::new(),
            runtime_config,
            secrets,
            facts_cache: None,
        })
    }
}
