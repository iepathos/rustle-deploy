//! Stat module for gathering file information

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

use crate::modules::error::{ModuleExecutionError, ValidationError};
use crate::modules::interface::{
    ArgumentSpec, ExecutionContext, ExecutionModule, ModuleArgs, ModuleDocumentation, ModuleResult,
    Platform, ReturnValueSpec,
};

use super::utils::{
    checksum::{calculate_file_checksum, ChecksumAlgorithm},
    ownership::get_ownership,
    permissions::get_permissions,
    FileError,
};

/// Stat module arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatArgs {
    pub path: String,                       // Required: path to examine
    pub follow: Option<bool>,               // Follow symlinks
    pub get_checksum: Option<bool>,         // Calculate file checksum
    pub checksum_algorithm: Option<String>, // sha1, sha256, md5
}

impl StatArgs {
    pub fn from_module_args(args: &ModuleArgs) -> Result<Self, ValidationError> {
        let mut stat_args = Self {
            path: String::new(),
            follow: None,
            get_checksum: None,
            checksum_algorithm: None,
        };

        // Required path
        if let Some(path) = args.args.get("path") {
            stat_args.path = path
                .as_str()
                .ok_or_else(|| ValidationError::InvalidArgValue {
                    arg: "path".to_string(),
                    value: "null".to_string(),
                    reason: "path must be a string".to_string(),
                })?
                .to_string();
        } else {
            return Err(ValidationError::MissingRequiredArg {
                arg: "path".to_string(),
            });
        }

        // Optional arguments
        if let Some(follow) = args.args.get("follow") {
            stat_args.follow = follow.as_bool();
        }

        if let Some(get_checksum) = args.args.get("get_checksum") {
            stat_args.get_checksum = get_checksum.as_bool();
        }

        if let Some(checksum_algorithm) = args.args.get("checksum_algorithm") {
            stat_args.checksum_algorithm = checksum_algorithm.as_str().map(|s| s.to_string());
        }

        Ok(stat_args)
    }
}

/// File stat result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatResult {
    pub exists: bool,
    pub path: String,
    pub mode: String,
    pub isdir: bool,
    pub isreg: bool,
    pub islnk: bool,
    pub size: u64,
    pub uid: u32,
    pub gid: u32,
    pub owner: String,
    pub group: String,
    pub mtime: f64,
    pub atime: f64,
    pub ctime: f64,
    pub checksum: Option<String>,
    pub link_target: Option<String>,
}

/// Stat module implementation
pub struct StatModule;

#[async_trait]
impl ExecutionModule for StatModule {
    fn name(&self) -> &'static str {
        "stat"
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

    async fn execute(
        &self,
        args: &ModuleArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        let stat_args =
            StatArgs::from_module_args(args).map_err(|e| ModuleExecutionError::Validation(e))?;

        self.execute_stat_operation(&stat_args, context).await
    }

    fn validate_args(&self, args: &ModuleArgs) -> Result<(), ValidationError> {
        StatArgs::from_module_args(args)?;
        Ok(())
    }

    async fn check_mode(
        &self,
        args: &ModuleArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        // Stat operation is read-only, so check mode is the same as regular execution
        self.execute(args, context).await
    }

    fn documentation(&self) -> ModuleDocumentation {
        ModuleDocumentation {
            description: "Retrieve file or filesystem status".to_string(),
            arguments: vec![
                ArgumentSpec {
                    name: "path".to_string(),
                    description: "Path to the file or directory to examine".to_string(),
                    required: true,
                    argument_type: "str".to_string(),
                    default: None,
                },
                ArgumentSpec {
                    name: "follow".to_string(),
                    description: "Follow symlinks".to_string(),
                    required: false,
                    argument_type: "bool".to_string(),
                    default: Some("false".to_string()),
                },
                ArgumentSpec {
                    name: "get_checksum".to_string(),
                    description: "Calculate file checksum".to_string(),
                    required: false,
                    argument_type: "bool".to_string(),
                    default: Some("false".to_string()),
                },
                ArgumentSpec {
                    name: "checksum_algorithm".to_string(),
                    description: "Checksum algorithm (sha1, sha256, md5)".to_string(),
                    required: false,
                    argument_type: "str".to_string(),
                    default: Some("sha256".to_string()),
                },
            ],
            examples: vec![
                r#"stat:
  path: /etc/passwd"#
                    .to_string(),
                r#"stat:
  path: /etc/ssl/cert.pem
  get_checksum: true
  checksum_algorithm: sha256"#
                    .to_string(),
            ],
            return_values: vec![
                ReturnValueSpec {
                    name: "exists".to_string(),
                    description: "Whether the path exists".to_string(),
                    returned: "always".to_string(),
                    value_type: "bool".to_string(),
                },
                ReturnValueSpec {
                    name: "path".to_string(),
                    description: "Path that was examined".to_string(),
                    returned: "always".to_string(),
                    value_type: "str".to_string(),
                },
                ReturnValueSpec {
                    name: "size".to_string(),
                    description: "File size in bytes".to_string(),
                    returned: "when path exists".to_string(),
                    value_type: "int".to_string(),
                },
                ReturnValueSpec {
                    name: "checksum".to_string(),
                    description: "File checksum".to_string(),
                    returned: "when get_checksum=true and file exists".to_string(),
                    value_type: "str".to_string(),
                },
            ],
        }
    }
}

impl StatModule {
    async fn execute_stat_operation(
        &self,
        args: &StatArgs,
        _context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        let path = Path::new(&args.path);
        let mut results = HashMap::new();

        if !path.exists() {
            // Path doesn't exist
            let stat_result = StatResult {
                exists: false,
                path: args.path.clone(),
                mode: "0000".to_string(),
                isdir: false,
                isreg: false,
                islnk: false,
                size: 0,
                uid: 0,
                gid: 0,
                owner: "".to_string(),
                group: "".to_string(),
                mtime: 0.0,
                atime: 0.0,
                ctime: 0.0,
                checksum: None,
                link_target: None,
            };

            results.insert(
                "stat".to_string(),
                serde_json::to_value(stat_result).map_err(|e| {
                    ModuleExecutionError::ExecutionFailed {
                        message: format!("Failed to serialize stat result: {}", e),
                    }
                })?,
            );

            return Ok(ModuleResult {
                changed: false,
                failed: false,
                msg: Some("Path does not exist".to_string()),
                stdout: None,
                stderr: None,
                rc: Some(0),
                results,
                diff: None,
                warnings: vec![],
                ansible_facts: HashMap::new(),
            });
        }

        // Get metadata (follow symlinks if requested)
        let metadata = if args.follow.unwrap_or(false) {
            fs::metadata(path).await
        } else {
            fs::symlink_metadata(path).await
        }
        .map_err(|e| ModuleExecutionError::ExecutionFailed {
            message: format!("Failed to get file metadata: {}", e),
        })?;

        // Get file type information
        let is_dir = metadata.is_dir();
        let is_file = metadata.is_file();
        let is_symlink = metadata.is_symlink();

        // Get timestamps
        let mtime = metadata
            .modified()
            .unwrap_or(std::time::UNIX_EPOCH)
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();

        let atime = metadata
            .accessed()
            .unwrap_or(std::time::UNIX_EPOCH)
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();

        let ctime = metadata
            .created()
            .unwrap_or(std::time::UNIX_EPOCH)
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();

        // Get permissions
        let mode = get_permissions(path)
            .await
            .unwrap_or_else(|_| "0000".to_string());

        // Get ownership
        let (owner, group) = get_ownership(path)
            .await
            .unwrap_or_else(|_| ("unknown".to_string(), "unknown".to_string()));

        // Platform-specific metadata
        #[cfg(unix)]
        let (uid, gid) = {
            use std::os::unix::fs::MetadataExt;
            (metadata.uid(), metadata.gid())
        };

        #[cfg(not(unix))]
        let (uid, gid) = (0, 0);

        // Get symlink target if it's a symlink
        let link_target = if is_symlink {
            fs::read_link(path)
                .await
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        } else {
            None
        };

        // Calculate checksum if requested and it's a regular file
        let checksum = if args.get_checksum.unwrap_or(false) && is_file {
            let algorithm = args
                .checksum_algorithm
                .as_ref()
                .map(|s| s.parse().unwrap_or(ChecksumAlgorithm::Sha256))
                .unwrap_or(ChecksumAlgorithm::Sha256);

            calculate_file_checksum(path, algorithm)
                .await
                .map_err(|e| ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to calculate checksum: {}", e),
                })?
                .into()
        } else {
            None
        };

        let stat_result = StatResult {
            exists: true,
            path: args.path.clone(),
            mode,
            isdir: is_dir,
            isreg: is_file,
            islnk: is_symlink,
            size: metadata.len(),
            uid,
            gid,
            owner,
            group,
            mtime,
            atime,
            ctime,
            checksum,
            link_target,
        };

        // Convert to JSON and add to results
        let stat_json = serde_json::to_value(stat_result).map_err(|e| {
            ModuleExecutionError::ExecutionFailed {
                message: format!("Failed to serialize stat result: {}", e),
            }
        })?;

        results.insert("stat".to_string(), stat_json);

        Ok(ModuleResult {
            changed: false, // Stat never changes anything
            failed: false,
            msg: Some("File information gathered successfully".to_string()),
            stdout: None,
            stderr: None,
            rc: Some(0),
            results,
            diff: None,
            warnings: vec![],
            ansible_facts: HashMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::interface::HostInfo;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tokio::io::AsyncWriteExt;

    fn create_test_context() -> ExecutionContext {
        ExecutionContext {
            facts: HashMap::new(),
            variables: HashMap::new(),
            host_info: HostInfo::detect(),
            working_directory: PathBuf::from("/tmp"),
            environment: HashMap::new(),
            check_mode: false,
            diff_mode: false,
            verbosity: 0,
        }
    }

    #[tokio::test]
    async fn test_stat_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file.txt");

        // Create test file
        let mut file = tokio::fs::File::create(&file_path).await.unwrap();
        file.write_all(b"test content").await.unwrap();
        file.flush().await.unwrap();

        let args = ModuleArgs {
            args: {
                let mut map = HashMap::new();
                map.insert(
                    "path".to_string(),
                    serde_json::Value::String(file_path.to_string_lossy().to_string()),
                );
                map.insert("get_checksum".to_string(), serde_json::Value::Bool(true));
                map
            },
            special: Default::default(),
        };

        let module = StatModule;
        let context = create_test_context();
        let result = module.execute(&args, &context).await.unwrap();

        assert!(!result.changed); // Stat never changes anything
        assert!(!result.failed);

        // Check that stat information is present
        let stat_value = result.results.get("stat").unwrap();
        let stat_result: StatResult = serde_json::from_value(stat_value.clone()).unwrap();

        assert!(stat_result.exists);
        assert!(stat_result.isreg);
        assert!(!stat_result.isdir);
        assert!(!stat_result.islnk);
        assert_eq!(stat_result.size, 12); // "test content" is 12 bytes
        assert!(stat_result.checksum.is_some());
    }

    #[tokio::test]
    async fn test_stat_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nonexistent.txt");

        let args = ModuleArgs {
            args: {
                let mut map = HashMap::new();
                map.insert(
                    "path".to_string(),
                    serde_json::Value::String(file_path.to_string_lossy().to_string()),
                );
                map
            },
            special: Default::default(),
        };

        let module = StatModule;
        let context = create_test_context();
        let result = module.execute(&args, &context).await.unwrap();

        assert!(!result.changed);
        assert!(!result.failed);

        let stat_value = result.results.get("stat").unwrap();
        let stat_result: StatResult = serde_json::from_value(stat_value.clone()).unwrap();

        assert!(!stat_result.exists);
    }

    #[tokio::test]
    async fn test_stat_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("test_dir");
        tokio::fs::create_dir(&dir_path).await.unwrap();

        let args = ModuleArgs {
            args: {
                let mut map = HashMap::new();
                map.insert(
                    "path".to_string(),
                    serde_json::Value::String(dir_path.to_string_lossy().to_string()),
                );
                map
            },
            special: Default::default(),
        };

        let module = StatModule;
        let context = create_test_context();
        let result = module.execute(&args, &context).await.unwrap();

        let stat_value = result.results.get("stat").unwrap();
        let stat_result: StatResult = serde_json::from_value(stat_value.clone()).unwrap();

        assert!(stat_result.exists);
        assert!(stat_result.isdir);
        assert!(!stat_result.isreg);
        assert!(!stat_result.islnk);
    }
}
