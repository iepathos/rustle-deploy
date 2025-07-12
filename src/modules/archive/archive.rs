//! Archive module for creating various archive formats

use crate::modules::{
    archive::{
        formats::{ArchiveDetector, ArchiveFormat, TarHandler, ZipHandler},
        utils::extraction::CreationResult,
    },
    error::{ModuleExecutionError, ValidationError},
    interface::{ExecutionContext, ExecutionModule, ModuleArgs, ModuleResult, Platform},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveArgs {
    pub path: Vec<String>,
    pub dest: String,
    pub format: Option<String>,
    pub exclude: Option<Vec<String>>,
    pub exclude_path: Option<Vec<String>>,
    #[serde(default)]
    pub compression_level: Option<u8>,
    #[serde(default)]
    pub remove: Option<bool>,
    pub mode: Option<String>,
    pub owner: Option<String>,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveResult {
    pub changed: bool,
    pub dest: String,
    pub archived_files: Vec<String>,
    pub total_size: u64,
    pub compressed_size: u64,
    pub compression_ratio: f64,
    pub format: String,
}

pub struct ArchiveModule;

impl ArchiveModule {
    pub fn new() -> Self {
        Self
    }

    async fn create_archive(
        &self,
        args: &ArchiveArgs,
        _context: &ExecutionContext,
    ) -> Result<ArchiveResult, ModuleExecutionError> {
        let dest_path = Path::new(&args.dest);
        let source_paths: Vec<PathBuf> = args.path.iter().map(PathBuf::from).collect();

        // Validate all source paths exist
        for path in &source_paths {
            if !path.exists() {
                return Err(ModuleExecutionError::ExecutionFailed {
                    message: format!("Source path does not exist: {}", path.display()),
                });
            }
        }

        // Determine format
        let format = self.determine_format(args, dest_path)?;

        // Filter source files based on exclude patterns
        let filtered_sources = self.filter_sources(&source_paths, args)?;

        // Create the archive
        let creation_result = match format {
            ArchiveFormat::Tar
            | ArchiveFormat::TarGz
            | ArchiveFormat::TarBz2
            | ArchiveFormat::TarXz => {
                let handler = TarHandler::new();
                handler
                    .create(
                        &filtered_sources,
                        dest_path,
                        &format,
                        args.compression_level,
                    )
                    .await
                    .map_err(|e| ModuleExecutionError::ExecutionFailed {
                        message: format!("TAR creation failed: {}", e),
                    })?;

                // Calculate results
                self.calculate_creation_result(&filtered_sources, dest_path, &format)
                    .await?
            }
            ArchiveFormat::Zip => {
                let handler = ZipHandler::new();
                handler
                    .create(&filtered_sources, dest_path, args.compression_level)
                    .await
                    .map_err(|e| ModuleExecutionError::ExecutionFailed {
                        message: format!("ZIP creation failed: {}", e),
                    })?;

                // Calculate results
                self.calculate_creation_result(&filtered_sources, dest_path, &format)
                    .await?
            }
            _ => {
                return Err(ModuleExecutionError::ExecutionFailed {
                    message: format!("Unsupported archive format for creation: {:?}", format),
                });
            }
        };

        // Set permissions and ownership on created archive
        if let Some(mode) = &args.mode {
            self.set_file_permissions(dest_path, mode)?;
        }

        if args.owner.is_some() || args.group.is_some() {
            self.set_file_ownership(dest_path, &args.owner, &args.group)?;
        }

        // Remove source files if requested
        if args.remove.unwrap_or(false) {
            for path in &filtered_sources {
                if path.is_file() {
                    tokio::fs::remove_file(path).await.map_err(|e| {
                        ModuleExecutionError::ExecutionFailed {
                            message: format!("Failed to remove source file: {}", e),
                        }
                    })?;
                } else if path.is_dir() {
                    tokio::fs::remove_dir_all(path).await.map_err(|e| {
                        ModuleExecutionError::ExecutionFailed {
                            message: format!("Failed to remove source directory: {}", e),
                        }
                    })?;
                }
            }
        }

        Ok(ArchiveResult {
            changed: true,
            dest: args.dest.clone(),
            archived_files: creation_result
                .archived_files
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
            total_size: creation_result.total_size,
            compressed_size: creation_result.compressed_size,
            compression_ratio: creation_result.compression_ratio(),
            format: format!("{:?}", format),
        })
    }

    fn determine_format(
        &self,
        args: &ArchiveArgs,
        dest_path: &Path,
    ) -> Result<ArchiveFormat, ModuleExecutionError> {
        if let Some(format_str) = &args.format {
            match format_str.to_lowercase().as_str() {
                "tar" => Ok(ArchiveFormat::Tar),
                "tar.gz" | "tgz" | "gzip" => Ok(ArchiveFormat::TarGz),
                "tar.bz2" | "tbz2" | "bzip2" => Ok(ArchiveFormat::TarBz2),
                "tar.xz" | "txz" | "xz" => Ok(ArchiveFormat::TarXz),
                "zip" => Ok(ArchiveFormat::Zip),
                _ => Err(ModuleExecutionError::ExecutionFailed {
                    message: format!("Unsupported format: {}", format_str),
                }),
            }
        } else {
            // Auto-detect from extension
            ArchiveDetector::detect_from_extension(dest_path).map_err(|e| {
                ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to detect format: {}", e),
                }
            })
        }
    }

    fn filter_sources(
        &self,
        source_paths: &[PathBuf],
        args: &ArchiveArgs,
    ) -> Result<Vec<PathBuf>, ModuleExecutionError> {
        let mut filtered = Vec::new();

        for path in source_paths {
            if self.should_include_path(path, args) {
                filtered.push(path.clone());
            }
        }

        if filtered.is_empty() {
            return Err(ModuleExecutionError::ExecutionFailed {
                message: "No files to archive after filtering".to_string(),
            });
        }

        Ok(filtered)
    }

    fn should_include_path(&self, path: &Path, args: &ArchiveArgs) -> bool {
        let path_str = path.to_string_lossy();

        // Check exclude patterns
        if let Some(exclude_patterns) = &args.exclude {
            for pattern in exclude_patterns {
                if glob_match(pattern, &path_str) {
                    return false;
                }
            }
        }

        // Check exclude paths
        if let Some(exclude_paths) = &args.exclude_path {
            for exclude_path in exclude_paths {
                if path.starts_with(exclude_path) {
                    return false;
                }
            }
        }

        true
    }

    async fn calculate_creation_result(
        &self,
        sources: &[PathBuf],
        dest_path: &Path,
        _format: &ArchiveFormat,
    ) -> Result<CreationResult, ModuleExecutionError> {
        let mut result = CreationResult::new(dest_path.to_path_buf());

        // Calculate total size of source files
        for source in sources {
            let size = self.calculate_path_size(source).await?;
            result.add_file(source.clone(), size);
        }

        // Get compressed size
        if dest_path.exists() {
            let compressed_size = tokio::fs::metadata(dest_path)
                .await
                .map_err(|e| ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to get archive size: {}", e),
                })?
                .len();
            result.set_compressed_size(compressed_size);
        }

        Ok(result)
    }

    async fn calculate_path_size(&self, path: &Path) -> Result<u64, ModuleExecutionError> {
        // Use blocking I/O for simplicity to avoid async recursion
        let path = path.to_path_buf();
        tokio::task::spawn_blocking(move || Self::calculate_path_size_sync(&path))
            .await
            .map_err(|e| ModuleExecutionError::ExecutionFailed {
                message: format!("Task join error: {}", e),
            })?
    }

    fn calculate_path_size_sync(path: &Path) -> Result<u64, ModuleExecutionError> {
        if path.is_file() {
            Ok(std::fs::metadata(path)
                .map_err(|e| ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to get file size: {}", e),
                })?
                .len())
        } else if path.is_dir() {
            let mut total_size = 0u64;
            let read_dir =
                std::fs::read_dir(path).map_err(|e| ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to read directory: {}", e),
                })?;

            for entry in read_dir {
                let entry = entry.map_err(|e| ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to read directory entry: {}", e),
                })?;
                total_size += Self::calculate_path_size_sync(&entry.path())?;
            }

            Ok(total_size)
        } else {
            Ok(0)
        }
    }

    fn set_file_permissions(&self, path: &Path, mode: &str) -> Result<(), ModuleExecutionError> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = u32::from_str_radix(mode, 8).map_err(|e| {
                ModuleExecutionError::ExecutionFailed {
                    message: format!("Invalid mode: {}", e),
                }
            })?;
            let permissions = std::fs::Permissions::from_mode(mode);
            std::fs::set_permissions(path, permissions).map_err(|e| {
                ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to set permissions: {}", e),
                }
            })?;
        }
        #[cfg(not(unix))]
        {
            tracing::warn!("Setting permissions not supported on this platform");
        }
        Ok(())
    }

    fn set_file_ownership(
        &self,
        path: &Path,
        owner: &Option<String>,
        group: &Option<String>,
    ) -> Result<(), ModuleExecutionError> {
        #[cfg(unix)]
        {
            use nix::unistd::{chown, Gid, Uid};

            let uid = if let Some(owner) = owner {
                Some(owner.parse::<u32>().map(Uid::from_raw).or_else(|_| {
                    nix::unistd::User::from_name(owner)
                        .map(|user| user.map(|u| u.uid))
                        .unwrap_or(None)
                        .ok_or_else(|| ModuleExecutionError::ExecutionFailed {
                            message: format!("Unknown user: {}", owner),
                        })
                })?)
            } else {
                None
            };

            let gid = if let Some(group) = group {
                Some(group.parse::<u32>().map(Gid::from_raw).or_else(|_| {
                    nix::unistd::Group::from_name(group)
                        .map(|group| group.map(|g| g.gid))
                        .unwrap_or(None)
                        .ok_or_else(|| ModuleExecutionError::ExecutionFailed {
                            message: format!("Unknown group: {}", group),
                        })
                })?)
            } else {
                None
            };

            chown(path, uid, gid).map_err(|e| ModuleExecutionError::ExecutionFailed {
                message: format!("Failed to change ownership: {}", e),
            })?;
        }
        #[cfg(not(unix))]
        {
            tracing::warn!("Setting ownership not supported on this platform");
        }
        Ok(())
    }
}

#[async_trait]
impl ExecutionModule for ArchiveModule {
    fn name(&self) -> &'static str {
        "archive"
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
            description: "Create compressed archives from files and directories".to_string(),
            arguments: vec![
                ArgumentSpec {
                    name: "path".to_string(),
                    description: "List of files/directories to archive".to_string(),
                    required: true,
                    argument_type: "list".to_string(),
                    default: None,
                },
                ArgumentSpec {
                    name: "dest".to_string(),
                    description: "Destination archive path".to_string(),
                    required: true,
                    argument_type: "string".to_string(),
                    default: None,
                },
            ],
            examples: vec!["archive:
  path: ['/path/to/files']
  dest: '/path/to/archive.tar.gz'"
                .to_string()],
            return_values: vec![ReturnValueSpec {
                name: "changed".to_string(),
                description: "Whether the archive was created".to_string(),
                returned: "always".to_string(),
                value_type: "boolean".to_string(),
            }],
        }
    }

    fn validate_args(&self, args: &ModuleArgs) -> Result<(), ValidationError> {
        let archive_args: ArchiveArgs = serde_json::from_value(serde_json::to_value(&args.args)?)
            .map_err(|e| ValidationError::InvalidArgValue {
            arg: "args".to_string(),
            value: "<complex>".to_string(),
            reason: e.to_string(),
        })?;

        if archive_args.path.is_empty() {
            return Err(ValidationError::MissingRequiredArg {
                arg: "path".to_string(),
            });
        }

        if archive_args.dest.is_empty() {
            return Err(ValidationError::MissingRequiredArg {
                arg: "dest".to_string(),
            });
        }

        // Validate compression level
        if let Some(level) = archive_args.compression_level {
            if level > 9 {
                return Err(ValidationError::InvalidArgValue {
                    arg: "compression_level".to_string(),
                    value: level.to_string(),
                    reason: "must be between 0 and 9".to_string(),
                });
            }
        }

        // Validate mode if provided
        if let Some(mode) = &archive_args.mode {
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
        let archive_args: ArchiveArgs = serde_json::from_value(serde_json::to_value(&args.args)?)
            .map_err(|e| ModuleExecutionError::InvalidArgs {
            message: e.to_string(),
        })?;

        let result = self.create_archive(&archive_args, context).await?;

        let mut results = HashMap::new();
        results.insert(
            "archive_result".to_string(),
            serde_json::to_value(result.clone()).unwrap(),
        );

        Ok(ModuleResult {
            changed: result.changed,
            failed: false,
            msg: Some(format!(
                "Created archive {} with {} files ({} bytes -> {} bytes, {:.1}% compression)",
                result.dest,
                result.archived_files.len(),
                result.total_size,
                result.compressed_size,
                result.compression_ratio * 100.0
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
        let archive_args: ArchiveArgs = serde_json::from_value(serde_json::to_value(&args.args)?)
            .map_err(|e| ModuleExecutionError::InvalidArgs {
            message: e.to_string(),
        })?;

        let source_paths: Vec<PathBuf> = archive_args.path.iter().map(PathBuf::from).collect();

        // Check if all sources exist
        let mut missing_sources = Vec::new();
        for path in &source_paths {
            if !path.exists() {
                missing_sources.push(path.to_string_lossy().to_string());
            }
        }

        if !missing_sources.is_empty() {
            return Ok(ModuleResult {
                changed: false,
                failed: true,
                msg: Some(format!(
                    "Source paths do not exist: {}",
                    missing_sources.join(", ")
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

        let dest_path = Path::new(&archive_args.dest);
        let would_change = !dest_path.exists() || archive_args.remove.unwrap_or(false);

        Ok(ModuleResult {
            changed: would_change,
            failed: false,
            msg: Some(format!(
                "Would create archive {} from {} source(s)",
                archive_args.dest,
                archive_args.path.len()
            )),
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

// Simple glob matching function
fn glob_match(pattern: &str, text: &str) -> bool {
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            let prefix = parts[0];
            let suffix = parts[1];
            return text.starts_with(prefix) && text.ends_with(suffix);
        }
    }
    pattern == text
}

impl Default for ArchiveModule {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::interface::ModuleArgs;

    #[test]
    fn test_module_validation() {
        let module = ArchiveModule::new();

        // Test valid args
        let valid_args_json = serde_json::json!({
            "path": ["/path/to/file1", "/path/to/file2"],
            "dest": "/path/to/archive.tar.gz"
        });
        let valid_args = ModuleArgs {
            args: serde_json::from_value(valid_args_json).unwrap(),
            special: crate::modules::interface::SpecialParameters::default(),
        };
        assert!(module.validate_args(&valid_args).is_ok());

        // Test empty path
        let invalid_args_json = serde_json::json!({
            "path": [],
            "dest": "/path/to/archive.tar.gz"
        });
        let invalid_args = ModuleArgs {
            args: serde_json::from_value(invalid_args_json).unwrap(),
            special: crate::modules::interface::SpecialParameters::default(),
        };
        assert!(module.validate_args(&invalid_args).is_err());

        // Test invalid compression level
        let invalid_args_json = serde_json::json!({
            "path": ["/path/to/file"],
            "dest": "/path/to/archive.tar.gz",
            "compression_level": 15
        });
        let invalid_args = ModuleArgs {
            args: serde_json::from_value(invalid_args_json).unwrap(),
            special: crate::modules::interface::SpecialParameters::default(),
        };
        assert!(module.validate_args(&invalid_args).is_err());
    }

    #[test]
    fn test_glob_matching() {
        assert!(glob_match("*.txt", "file.txt"));
        assert!(glob_match("test*", "test123"));
        assert!(!glob_match("*.txt", "file.log"));
        assert!(glob_match("exact", "exact"));
    }
}
