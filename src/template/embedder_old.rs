use crate::execution::rustle_plan::{RustlePlanOutput, BinaryDeploymentPlan, StaticFileRef, SecretRef};
use crate::runtime::RuntimeConfig;
use anyhow::{Context, Result};
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

use super::{CompressionType, EmbeddedData, EncryptedSecrets, TargetInfo, TemplateConfig};

#[derive(Error, Debug)]
pub enum EmbedError {
    #[error("Compression failed: {0}")]
    Compression(String),
    #[error("Encryption failed: {0}")]
    Encryption(String),
    #[error("File operation failed: {0}")]
    FileOperation(#[from] std::io::Error),
    #[error("Serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Data embedding system for static files, modules, and secrets
pub struct DataEmbedder {
    encryptor: SecretEncryptor,
    compressor: DataCompressor,
}

impl DataEmbedder {
    pub fn new(config: &TemplateConfig) -> Result<Self> {
        let encryptor = SecretEncryptor::new(config.encrypt_secrets);
        let compressor = DataCompressor::new(config.compression_algorithm.clone());
        
        Ok(Self {
            encryptor,
            compressor,
        })
    }

    /// Embed complete execution data into template-ready format
    pub async fn embed_execution_data(
        &self,
        execution_plan: &RustlePlanOutput,
        binary_deployment: &BinaryDeploymentPlan,
        target_info: &TargetInfo,
    ) -> Result<EmbeddedData, EmbedError> {
        // Serialize execution plan
        let execution_plan_json = serde_json::to_string_pretty(execution_plan)?;
        
        // Convert static file refs to static files and embed them
        let static_files_converted: Vec<StaticFile> = binary_deployment.static_files.iter()
            .map(|file_ref| StaticFile {
                source_path: std::path::PathBuf::from(&file_ref.source_path),
                embedded_path: file_ref.target_path.clone(),
                content: std::fs::read(&file_ref.source_path).unwrap_or_default(),
                permissions: file_ref.permissions.unwrap_or(0o644),
                compression: if file_ref.compress { 
                    self.compressor.algorithm.clone() 
                } else { 
                    CompressionType::None 
                },
            })
            .collect();
        
        let static_files = self.embed_static_files(&static_files_converted).await?;
        
        // Create runtime configuration
        let runtime_config = RuntimeConfig {
            controller_endpoint: binary_deployment.controller_endpoint.clone(),
            execution_timeout: binary_deployment.execution_timeout,
            report_interval: binary_deployment.report_interval.unwrap_or(std::time::Duration::from_secs(30)),
            cleanup_on_completion: binary_deployment.cleanup_on_completion.unwrap_or(true),
            log_level: binary_deployment.log_level.clone().unwrap_or_else(|| "info".to_string()),
            heartbeat_interval: std::time::Duration::from_secs(60),
            max_retries: binary_deployment.max_retries.unwrap_or(3),
        };

        // Embed modules (placeholder for now)
        let module_binaries = HashMap::new();
        
        // Convert secret refs to secret specs and encrypt them
        let secret_specs: Vec<SecretSpec> = binary_deployment.secrets.iter()
            .map(|secret_ref| {
                let value = match &secret_ref.source {
                    crate::execution::rustle_plan::SecretSource::File { path } => {
                        std::fs::read(path).unwrap_or_default()
                    },
                    crate::execution::rustle_plan::SecretSource::Environment { var } => {
                        std::env::var(var).unwrap_or_default().into_bytes()
                    },
                    crate::execution::rustle_plan::SecretSource::Vault { .. } => {
                        // Vault integration would go here
                        Vec::new()
                    },
                };
                
                SecretSpec {
                    key: secret_ref.key.clone(),
                    value,
                    secret_type: SecretType::Token, // Default type
                }
            })
            .collect();
            
        let secrets = self.embed_secrets(&secret_specs, target_info).await?;

        Ok(EmbeddedData {
            execution_plan: execution_plan_json,
            static_files,
            module_binaries,
            runtime_config,
            secrets,
            facts_cache: None,
        })
    }

    /// Embed execution plan as compile-time data
    pub fn embed_execution_plan(&self, plan: &RustlePlanOutput) -> Result<String, EmbedError> {
        let plan_json = serde_json::to_string_pretty(plan)?;
        
        if self.compressor.should_compress(&plan_json.as_bytes()) {
            let compressed = self.compressor.compress(plan_json.as_bytes())?;
            Ok(format!(
                r#"const EXECUTION_PLAN_COMPRESSED: &[u8] = &{:?};
const EXECUTION_PLAN: &str = include_str!(concat!(env!("OUT_DIR"), "/execution_plan.json"));"#,
                compressed
            ))
        } else {
            Ok(format!(
                r#"const EXECUTION_PLAN: &str = r##"{}"##;"#,
                plan_json
            ))
        }
    }

    /// Embed static files with compression
    pub async fn embed_static_files(&self, files: &[StaticFile]) -> Result<HashMap<String, Vec<u8>>, EmbedError> {
        let mut embedded_files = HashMap::new();
        
        for file in files {
            let content = if self.compressor.should_compress(&file.content) {
                self.compressor.compress(&file.content)?
            } else {
                file.content.clone()
            };
            
            embedded_files.insert(file.embedded_path.clone(), content);
        }
        
        Ok(embedded_files)
    }

    /// Embed compiled modules
    pub fn embed_modules(&self, modules: &[CompiledModule]) -> Result<HashMap<String, String>, EmbedError> {
        let mut embedded_modules = HashMap::new();
        
        for module in modules {
            let module_code = format!(
                r#"pub mod {} {{
    const MODULE_BINARY: &[u8] = &{:?};
    
    pub fn get_binary() -> &'static [u8] {{
        MODULE_BINARY
    }}
}}"#,
                module.name.replace(":", "_"),
                module.binary
            );
            
            embedded_modules.insert(
                format!("{}.rs", module.name.replace(":", "_")),
                module_code,
            );
        }
        
        Ok(embedded_modules)
    }

    /// Embed secrets with encryption
    pub async fn embed_secrets(
        &self,
        secrets: &[SecretSpec],
        target_info: &TargetInfo,
    ) -> Result<EncryptedSecrets, EmbedError> {
        let mut vault_data = HashMap::new();
        
        for secret in secrets {
            let encrypted_data = self.encryptor.encrypt(&secret.value, &target_info.target_triple).await?;
            vault_data.insert(secret.key.clone(), encrypted_data);
        }
        
        Ok(EncryptedSecrets {
            vault_data,
            encryption_key_id: self.encryptor.get_key_id(),
            decryption_method: self.encryptor.get_method(),
        })
    }

    /// Generate code for accessing embedded data at runtime
    pub fn generate_embedded_data_accessors(&self, embedded_data: &EmbeddedData) -> Result<String, EmbedError> {
        let static_file_accessors = embedded_data.static_files
            .keys()
            .map(|path| {
                let safe_name = path.replace("/", "_").replace(".", "_").replace("-", "_");
                format!(
                    r#"    pub fn get_{}() -> &'static [u8] {{
        STATIC_FILES.get("{}").copied().unwrap_or(&[])
    }}"#,
                    safe_name, path
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(format!(
            r#"pub mod embedded_data {{
    use std::collections::HashMap;
    use once_cell::sync::Lazy;
    
    static STATIC_FILES: Lazy<HashMap<&'static str, &'static [u8]>> = Lazy::new(|| {{
        let mut files = HashMap::new();
{}
        files
    }});
    
    pub fn get_execution_plan() -> &'static str {{
        EXECUTION_PLAN
    }}
    
    pub fn get_runtime_config() -> &'static str {{
        RUNTIME_CONFIG
    }}
    
{}
}}"#,
            embedded_data.static_files
                .iter()
                .map(|(path, _)| format!(
                    r#"        files.insert("{}", include_bytes!(concat!(env!("OUT_DIR"), "/static_files/{}")));"#,
                    path, path.replace("/", "_")
                ))
                .collect::<Vec<_>>()
                .join("\n"),
            static_file_accessors
        ))
    }
}

/// Handles compression of embedded data
pub struct DataCompressor {
    algorithm: CompressionType,
}

impl DataCompressor {
    pub fn new(algorithm: CompressionType) -> Self {
        Self { algorithm }
    }

    pub fn should_compress(&self, data: &[u8]) -> bool {
        match self.algorithm {
            CompressionType::None => false,
            _ => data.len() > 1024, // Compress files larger than 1KB
        }
    }

    pub fn compress(&self, data: &[u8]) -> Result<Vec<u8>, EmbedError> {
        match self.algorithm {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Gzip => {
                use flate2::write::GzEncoder;
                use std::io::Write;
                
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(data)
                    .map_err(|e| EmbedError::Compression(e.to_string()))?;
                encoder.finish()
                    .map_err(|e| EmbedError::Compression(e.to_string()))
            },
            CompressionType::Lz4 => {
                // Would use lz4 crate here
                Err(EmbedError::Compression("LZ4 compression not implemented".to_string()))
            },
            CompressionType::Zstd => {
                // Would use zstd crate here
                Err(EmbedError::Compression("Zstd compression not implemented".to_string()))
            },
        }
    }

    pub fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, EmbedError> {
        match self.algorithm {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Gzip => {
                use flate2::read::GzDecoder;
                use std::io::Read;
                
                let mut decoder = GzDecoder::new(data);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)
                    .map_err(|e| EmbedError::Compression(e.to_string()))?;
                Ok(decompressed)
            },
            CompressionType::Lz4 => {
                Err(EmbedError::Compression("LZ4 decompression not implemented".to_string()))
            },
            CompressionType::Zstd => {
                Err(EmbedError::Compression("Zstd decompression not implemented".to_string()))
            },
        }
    }
}

/// Handles encryption of sensitive data
pub struct SecretEncryptor {
    enabled: bool,
    key_id: String,
}

impl SecretEncryptor {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            key_id: if enabled {
                uuid::Uuid::new_v4().to_string()
            } else {
                "none".to_string()
            },
        }
    }

    pub async fn encrypt(&self, data: &[u8], _target_triple: &str) -> Result<Vec<u8>, EmbedError> {
        if !self.enabled {
            return Ok(data.to_vec());
        }

        // For now, just return the data as-is
        // In a real implementation, this would use proper encryption
        // with target-specific keys
        Ok(data.to_vec())
    }

    pub fn get_key_id(&self) -> String {
        self.key_id.clone()
    }

    pub fn get_method(&self) -> String {
        if self.enabled {
            "aes-gcm".to_string()
        } else {
            "none".to_string()
        }
    }
}

// Supporting types

#[derive(Debug, Clone)]
pub struct StaticFile {
    pub source_path: PathBuf,
    pub embedded_path: String,
    pub content: Vec<u8>,
    pub permissions: u32,
    pub compression: CompressionType,
}

#[derive(Debug, Clone)]
pub struct CompiledModule {
    pub name: String,
    pub binary: Vec<u8>,
    pub target_triple: String,
}

#[derive(Debug, Clone)]
pub struct SecretSpec {
    pub key: String,
    pub value: Vec<u8>,
    pub secret_type: SecretType,
}

#[derive(Debug, Clone)]
pub enum SecretType {
    ApiKey,
    Certificate,
    PrivateKey,
    Password,
    Token,
}