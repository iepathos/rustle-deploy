//! Unarchive module for extracting various archive formats

use crate::modules::{
    archive::{
        formats::{ArchiveDetector, ArchiveFormat, TarHandler, ZipHandler},
        utils::extraction::ExtractionOptions,
    },
    error::{ModuleExecutionError, ValidationError},
    interface::{ExecutionContext, ExecutionModule, ModuleArgs, ModuleResult, Platform},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnarchiveArgs {
    pub src: String,
    pub dest: String,
    #[serde(default)]
    pub remote_src: Option<bool>,
    pub creates: Option<String>,
    #[serde(default)]
    pub list_files: Option<bool>,
    pub exclude: Option<Vec<String>>,
    pub include: Option<Vec<String>>,
    #[serde(default)]
    pub keep_newer: Option<bool>,
    pub mode: Option<String>,
    pub owner: Option<String>,
    pub group: Option<String>,
    #[serde(default)]
    pub validate_certs: Option<bool>,
    pub checksum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnarchiveResult {
    pub changed: bool,
    pub dest: String,
    pub extracted_files: Option<Vec<String>>,
    pub total_size: u64,
    pub format: String,
}

pub struct UnarchiveModule;

impl UnarchiveModule {
    pub fn new() -> Self {
        Self
    }

    async fn extract_archive(
        &self,
        args: &UnarchiveArgs,
        _context: &ExecutionContext,
    ) -> Result<UnarchiveResult, ModuleExecutionError> {
        let src_path = Path::new(&args.src);
        let dest_path = Path::new(&args.dest);

        // Check if we should skip extraction because target already exists
        if let Some(creates) = &args.creates {
            let creates_path = Path::new(creates);
            if creates_path.exists() {
                return Ok(UnarchiveResult {
                    changed: false,
                    dest: args.dest.clone(),
                    extracted_files: None,
                    total_size: 0,
                    format: "skipped".to_string(),
                });
            }
        }

        // Validate checksum if provided
        if let Some(expected_checksum) = &args.checksum {
            self.validate_checksum(src_path, expected_checksum).await?;
        }

        // Detect archive format
        let format = self.detect_format(src_path).await?;

        // Prepare extraction options
        let options = ExtractionOptions {
            exclude: args.exclude.clone(),
            include: args.include.clone(),
            keep_newer: args.keep_newer.unwrap_or(false),
            mode: args.mode.clone(),
            owner: args.owner.clone(),
            group: args.group.clone(),
        };

        // Extract based on format
        let extraction_result = match format {
            ArchiveFormat::Tar
            | ArchiveFormat::TarGz
            | ArchiveFormat::TarBz2
            | ArchiveFormat::TarXz => {
                let handler = TarHandler::new();
                handler
                    .extract(src_path, dest_path, &format, &options)
                    .await
                    .map_err(|e| ModuleExecutionError::ExecutionFailed {
                        message: format!("TAR extraction failed: {e}"),
                    })?
            }
            ArchiveFormat::Zip => {
                let handler = ZipHandler::new();
                handler
                    .extract(src_path, dest_path, &options)
                    .await
                    .map_err(|e| ModuleExecutionError::ExecutionFailed {
                        message: format!("ZIP extraction failed: {e}"),
                    })?
            }
            _ => {
                return Err(ModuleExecutionError::ExecutionFailed {
                    message: format!("Unsupported archive format: {format:?}"),
                });
            }
        };

        let extracted_files = if args.list_files.unwrap_or(false) {
            Some(
                extraction_result
                    .extracted_files
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect(),
            )
        } else {
            None
        };

        Ok(UnarchiveResult {
            changed: !extraction_result.extracted_files.is_empty(),
            dest: args.dest.clone(),
            extracted_files,
            total_size: extraction_result.total_size,
            format: format!("{format:?}"),
        })
    }

    async fn detect_format(&self, path: &Path) -> Result<ArchiveFormat, ModuleExecutionError> {
        // First try extension-based detection
        if let Ok(format) = ArchiveDetector::detect_from_extension(path) {
            if ArchiveDetector::is_extraction_supported(&format) {
                return Ok(format);
            }
        }

        // Fall back to magic byte detection
        let file = tokio::fs::File::open(path).await.map_err(|e| {
            ModuleExecutionError::ExecutionFailed {
                message: format!("Failed to open archive: {e}"),
            }
        })?;

        let mut reader = BufReader::new(file.into_std().await);

        ArchiveDetector::detect_from_magic_bytes(&mut reader).map_err(|e| {
            ModuleExecutionError::ExecutionFailed {
                message: format!("Failed to detect archive format: {e}"),
            }
        })
    }

    async fn validate_checksum(
        &self,
        path: &Path,
        expected: &str,
    ) -> Result<(), ModuleExecutionError> {
        use md5::Md5;
        use sha1::Sha1;
        use sha2::{Digest, Sha256};

        // Parse checksum format: "algo:hash" or just "hash" (assume SHA256)
        let (algorithm, expected_hash) = if expected.contains(':') {
            let parts: Vec<&str> = expected.splitn(2, ':').collect();
            (parts[0], parts[1])
        } else {
            ("sha256", expected)
        };

        let file_content =
            tokio::fs::read(path)
                .await
                .map_err(|e| ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to read file for checksum: {e}"),
                })?;

        let actual_hash = match algorithm.to_lowercase().as_str() {
            "md5" => {
                let mut hasher = Md5::new();
                hasher.update(&file_content);
                format!("{:x}", hasher.finalize())
            }
            "sha1" => {
                let mut hasher = Sha1::new();
                hasher.update(&file_content);
                format!("{:x}", hasher.finalize())
            }
            "sha256" => {
                let mut hasher = Sha256::new();
                hasher.update(&file_content);
                format!("{:x}", hasher.finalize())
            }
            _ => {
                return Err(ModuleExecutionError::ExecutionFailed {
                    message: format!("Unsupported checksum algorithm: {algorithm}"),
                });
            }
        };

        if actual_hash != expected_hash {
            return Err(ModuleExecutionError::ExecutionFailed {
                message: format!(
                    "Checksum mismatch. Expected: {expected_hash}, Actual: {actual_hash}"
                ),
            });
        }

        Ok(())
    }
}

#[async_trait]
impl ExecutionModule for UnarchiveModule {
    fn name(&self) -> &'static str {
        "unarchive"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn supported_platforms(&self) -> &[Platform] {
        &[
            Platform::Linux,
            Platform::MacOS,
            Platform::Windows,
            Platform::FreeBSD,
            Platform::OpenBSD,
            Platform::NetBSD,
        ]
    }

    fn documentation(&self) -> crate::modules::interface::ModuleDocumentation {
        use crate::modules::interface::{ArgumentSpec, ModuleDocumentation, ReturnValueSpec};

        ModuleDocumentation {
            description: "Extract files from compressed archives".to_string(),
            arguments: vec![
                ArgumentSpec {
                    name: "src".to_string(),
                    description: "Source archive path".to_string(),
                    required: true,
                    argument_type: "string".to_string(),
                    default: None,
                },
                ArgumentSpec {
                    name: "dest".to_string(),
                    description: "Destination directory".to_string(),
                    required: true,
                    argument_type: "string".to_string(),
                    default: None,
                },
            ],
            examples: vec!["unarchive:
  src: '/path/to/archive.tar.gz'
  dest: '/path/to/extract'"
                .to_string()],
            return_values: vec![ReturnValueSpec {
                name: "changed".to_string(),
                description: "Whether files were extracted".to_string(),
                returned: "always".to_string(),
                value_type: "boolean".to_string(),
            }],
        }
    }

    fn validate_args(&self, args: &ModuleArgs) -> Result<(), ValidationError> {
        let unarchive_args: UnarchiveArgs =
            serde_json::from_value(serde_json::to_value(&args.args)?).map_err(|e| {
                ValidationError::InvalidArgValue {
                    arg: "args".to_string(),
                    value: "<complex>".to_string(),
                    reason: e.to_string(),
                }
            })?;

        if unarchive_args.src.is_empty() {
            return Err(ValidationError::MissingRequiredArg {
                arg: "src".to_string(),
            });
        }

        if unarchive_args.dest.is_empty() {
            return Err(ValidationError::MissingRequiredArg {
                arg: "dest".to_string(),
            });
        }

        // Validate mode if provided
        if let Some(mode) = &unarchive_args.mode {
            if !crate::modules::archive::utils::extraction::utils::validate_permissions(mode) {
                return Err(ValidationError::InvalidArgValue {
                    arg: "mode".to_string(),
                    value: mode.to_string(),
                    reason: "invalid file permissions format".to_string(),
                });
            }
        }

        Ok(())
    }

    async fn execute(
        &self,
        args: &ModuleArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        let unarchive_args: UnarchiveArgs =
            serde_json::from_value(serde_json::to_value(&args.args)?).map_err(|e| {
                ModuleExecutionError::InvalidArgs {
                    message: e.to_string(),
                }
            })?;

        let result = self.extract_archive(&unarchive_args, context).await?;

        let mut results = HashMap::new();
        results.insert(
            "unarchive_result".to_string(),
            serde_json::to_value(result.clone()).unwrap(),
        );

        Ok(ModuleResult {
            changed: result.changed,
            failed: false,
            msg: Some(format!(
                "Extracted {} files ({} bytes) from {} to {}",
                result
                    .extracted_files
                    .as_ref()
                    .map(|f| f.len())
                    .unwrap_or(0),
                result.total_size,
                unarchive_args.src,
                result.dest
            )),
            stdout: None,
            stderr: None,
            rc: Some(0),
            results,
            diff: None,
            warnings: vec![],
            ansible_facts: HashMap::new(),
        })
    }

    async fn check_mode(
        &self,
        args: &ModuleArgs,
        _context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        let unarchive_args: UnarchiveArgs =
            serde_json::from_value(serde_json::to_value(&args.args)?).map_err(|e| {
                ModuleExecutionError::InvalidArgs {
                    message: e.to_string(),
                }
            })?;

        let src_path = Path::new(&unarchive_args.src);
        let _dest_path = Path::new(&unarchive_args.dest);

        // Check if source exists
        if !src_path.exists() {
            return Ok(ModuleResult {
                changed: false,
                failed: true,
                msg: Some(format!(
                    "Source archive does not exist: {}",
                    unarchive_args.src
                )),
                stdout: None,
                stderr: None,
                rc: Some(1),
                results: HashMap::new(),
                diff: None,
                warnings: vec![],
                ansible_facts: HashMap::new(),
            });
        }

        // Check if we would skip due to 'creates'
        let would_skip = if let Some(creates) = &unarchive_args.creates {
            Path::new(creates).exists()
        } else {
            false
        };

        let would_change = !would_skip && src_path.exists();

        Ok(ModuleResult {
            changed: would_change,
            failed: false,
            msg: Some(if would_skip {
                format!(
                    "Would skip extraction because {} already exists",
                    unarchive_args.creates.unwrap()
                )
            } else {
                format!(
                    "Would extract archive from {} to {}",
                    unarchive_args.src, unarchive_args.dest
                )
            }),
            stdout: None,
            stderr: None,
            rc: Some(0),
            results: HashMap::new(),
            diff: None,
            warnings: vec![],
            ansible_facts: HashMap::new(),
        })
    }
}

impl Default for UnarchiveModule {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::interface::ModuleArgs;

    #[tokio::test]
    async fn test_module_validation() {
        let module = UnarchiveModule::new();

        // Test valid args
        let valid_args_json = serde_json::json!({
            "src": "/path/to/archive.tar.gz",
            "dest": "/path/to/dest"
        });
        let valid_args = ModuleArgs {
            args: serde_json::from_value(valid_args_json).unwrap(),
            special: crate::modules::interface::SpecialParameters::default(),
        };
        assert!(module.validate_args(&valid_args).is_ok());

        // Test missing src
        let invalid_args_json = serde_json::json!({
            "dest": "/path/to/dest"
        });
        let invalid_args = ModuleArgs {
            args: serde_json::from_value(invalid_args_json).unwrap(),
            special: crate::modules::interface::SpecialParameters::default(),
        };
        assert!(module.validate_args(&invalid_args).is_err());

        // Test empty src
        let invalid_args_json = serde_json::json!({
            "src": "",
            "dest": "/path/to/dest"
        });
        let invalid_args = ModuleArgs {
            args: serde_json::from_value(invalid_args_json).unwrap(),
            special: crate::modules::interface::SpecialParameters::default(),
        };
        assert!(module.validate_args(&invalid_args).is_err());
    }

    #[test]
    fn test_archive_args_deserialization() {
        let json = serde_json::json!({
            "src": "/path/to/archive.tar.gz",
            "dest": "/path/to/dest",
            "exclude": ["*.log", "tmp/*"],
            "mode": "755"
        });

        let args: UnarchiveArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.src, "/path/to/archive.tar.gz");
        assert_eq!(args.dest, "/path/to/dest");
        assert_eq!(
            args.exclude,
            Some(vec!["*.log".to_string(), "tmp/*".to_string()])
        );
        assert_eq!(args.mode, Some("755".to_string()));
    }
}
