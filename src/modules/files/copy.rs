//! Copy module for file copying operations

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tokio::process::Command;

use crate::modules::error::{ModuleExecutionError, ValidationError};
use crate::modules::interface::{
    ArgumentSpec, ExecutionContext, ExecutionModule, ModuleArgs, ModuleDocumentation, ModuleResult,
    Platform, ReturnValueSpec,
};

use super::utils::{
    atomic::AtomicWriter,
    backup::create_backup,
    checksum::{verify_file_checksum, ChecksumAlgorithm},
    ownership::set_ownership,
    permissions::{get_permissions, set_permissions},
};

/// Copy module arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyArgs {
    pub src: String,                    // Required: source file path
    pub dest: String,                   // Required: destination path
    pub backup: Option<bool>,           // Create backup of destination
    pub force: Option<bool>,            // Overwrite existing files
    pub mode: Option<String>,           // Set permissions on copied file
    pub owner: Option<String>,          // Set owner on copied file
    pub group: Option<String>,          // Set group on copied file
    pub directory_mode: Option<String>, // Permissions for created directories
    pub validate: Option<String>,       // Command to validate copied file
    pub checksum: Option<String>,       // Expected checksum of source
    pub preserve: Option<bool>,         // Preserve source file attributes
}

impl CopyArgs {
    pub fn from_module_args(args: &ModuleArgs) -> Result<Self, ValidationError> {
        let mut copy_args = Self {
            src: String::new(),
            dest: String::new(),
            backup: None,
            force: None,
            mode: None,
            owner: None,
            group: None,
            directory_mode: None,
            validate: None,
            checksum: None,
            preserve: None,
        };

        // Required src
        if let Some(src) = args.args.get("src") {
            copy_args.src = src
                .as_str()
                .ok_or_else(|| ValidationError::InvalidArgValue {
                    arg: "src".to_string(),
                    value: "null".to_string(),
                    reason: "src must be a string".to_string(),
                })?
                .to_string();
        } else {
            return Err(ValidationError::MissingRequiredArg {
                arg: "src".to_string(),
            });
        }

        // Required dest
        if let Some(dest) = args.args.get("dest") {
            copy_args.dest = dest
                .as_str()
                .ok_or_else(|| ValidationError::InvalidArgValue {
                    arg: "dest".to_string(),
                    value: "null".to_string(),
                    reason: "dest must be a string".to_string(),
                })?
                .to_string();
        } else {
            return Err(ValidationError::MissingRequiredArg {
                arg: "dest".to_string(),
            });
        }

        // Optional arguments
        if let Some(backup) = args.args.get("backup") {
            copy_args.backup = backup.as_bool();
        }

        if let Some(force) = args.args.get("force") {
            copy_args.force = force.as_bool();
        }

        if let Some(mode) = args.args.get("mode") {
            copy_args.mode = mode.as_str().map(|s| s.to_string());
        }

        if let Some(owner) = args.args.get("owner") {
            copy_args.owner = owner.as_str().map(|s| s.to_string());
        }

        if let Some(group) = args.args.get("group") {
            copy_args.group = group.as_str().map(|s| s.to_string());
        }

        if let Some(directory_mode) = args.args.get("directory_mode") {
            copy_args.directory_mode = directory_mode.as_str().map(|s| s.to_string());
        }

        if let Some(validate) = args.args.get("validate") {
            copy_args.validate = validate.as_str().map(|s| s.to_string());
        }

        if let Some(checksum) = args.args.get("checksum") {
            copy_args.checksum = checksum.as_str().map(|s| s.to_string());
        }

        if let Some(preserve) = args.args.get("preserve") {
            copy_args.preserve = preserve.as_bool();
        }

        Ok(copy_args)
    }
}

/// Copy module implementation
pub struct CopyModule;

#[async_trait]
impl ExecutionModule for CopyModule {
    fn name(&self) -> &'static str {
        "copy"
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
        let copy_args =
            CopyArgs::from_module_args(args).map_err(ModuleExecutionError::Validation)?;

        self.execute_copy_operation(&copy_args, context).await
    }

    fn validate_args(&self, args: &ModuleArgs) -> Result<(), ValidationError> {
        CopyArgs::from_module_args(args)?;
        Ok(())
    }

    async fn check_mode(
        &self,
        args: &ModuleArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        let copy_args =
            CopyArgs::from_module_args(args).map_err(ModuleExecutionError::Validation)?;

        self.analyze_copy_operation(&copy_args, context).await
    }

    fn documentation(&self) -> ModuleDocumentation {
        ModuleDocumentation {
            description: "Copy files from source to destination".to_string(),
            arguments: vec![
                ArgumentSpec {
                    name: "src".to_string(),
                    description: "Source file path".to_string(),
                    required: true,
                    argument_type: "str".to_string(),
                    default: None,
                },
                ArgumentSpec {
                    name: "dest".to_string(),
                    description: "Destination file path".to_string(),
                    required: true,
                    argument_type: "str".to_string(),
                    default: None,
                },
                ArgumentSpec {
                    name: "backup".to_string(),
                    description: "Create backup of destination file".to_string(),
                    required: false,
                    argument_type: "bool".to_string(),
                    default: Some("false".to_string()),
                },
                ArgumentSpec {
                    name: "force".to_string(),
                    description: "Overwrite destination file if it exists".to_string(),
                    required: false,
                    argument_type: "bool".to_string(),
                    default: Some("false".to_string()),
                },
                ArgumentSpec {
                    name: "mode".to_string(),
                    description: "Set permissions on destination file".to_string(),
                    required: false,
                    argument_type: "str".to_string(),
                    default: None,
                },
                ArgumentSpec {
                    name: "validate".to_string(),
                    description:
                        "Command to validate copied file (%s will be replaced with file path)"
                            .to_string(),
                    required: false,
                    argument_type: "str".to_string(),
                    default: None,
                },
                ArgumentSpec {
                    name: "checksum".to_string(),
                    description: "Expected checksum of source file".to_string(),
                    required: false,
                    argument_type: "str".to_string(),
                    default: None,
                },
            ],
            examples: vec![r#"copy:
  src: /etc/example.conf
  dest: /etc/myapp/myapp.conf
  backup: yes
  mode: '0644'"#
                .to_string()],
            return_values: vec![
                ReturnValueSpec {
                    name: "changed".to_string(),
                    description: "Whether the file was copied".to_string(),
                    returned: "always".to_string(),
                    value_type: "bool".to_string(),
                },
                ReturnValueSpec {
                    name: "src".to_string(),
                    description: "Source file path".to_string(),
                    returned: "always".to_string(),
                    value_type: "str".to_string(),
                },
                ReturnValueSpec {
                    name: "dest".to_string(),
                    description: "Destination file path".to_string(),
                    returned: "always".to_string(),
                    value_type: "str".to_string(),
                },
            ],
        }
    }
}

impl CopyModule {
    async fn execute_copy_operation(
        &self,
        args: &CopyArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        // If we're in check mode, delegate to the analyze method
        if context.check_mode {
            return self.analyze_copy_operation(args, context).await;
        }
        let src_path = Path::new(&args.src);
        let original_dest_path = Path::new(&args.dest);
        #[allow(unused_assignments)]
        let mut changed = false;
        let mut results = HashMap::new();

        // Check if source exists
        if !src_path.exists() {
            return Err(ModuleExecutionError::ExecutionFailed {
                message: format!("Source file does not exist: {}", args.src),
            });
        }

        // Handle destination path based on whether it's a directory
        let dest_path = self.resolve_destination_path(src_path, original_dest_path)?;

        // Verify checksum if provided (only for files)
        if let Some(expected_checksum) = &args.checksum {
            if src_path.is_file() {
                let is_valid =
                    verify_file_checksum(src_path, expected_checksum, ChecksumAlgorithm::Sha256)
                        .await
                        .map_err(|e| ModuleExecutionError::ExecutionFailed {
                            message: format!("Checksum verification failed: {e}"),
                        })?;

                if !is_valid {
                    return Err(ModuleExecutionError::ExecutionFailed {
                        message: "Source file checksum does not match expected value".to_string(),
                    });
                }
            }
        }

        // Perform the copy operation based on source type
        changed = if src_path.is_dir() {
            self.copy_directory(src_path, &dest_path, args).await?
        } else {
            self.copy_file(src_path, &dest_path, args).await?
        };

        // Run validation command if specified (only for files)
        if let Some(validate_cmd) = &args.validate {
            if src_path.is_file() {
                let cmd = validate_cmd.replace("%s", &dest_path.to_string_lossy());
                let output = Command::new("sh")
                    .arg("-c")
                    .arg(&cmd)
                    .output()
                    .await
                    .map_err(|e| ModuleExecutionError::ExecutionFailed {
                        message: format!("Failed to run validation command: {e}"),
                    })?;

                if !output.status.success() {
                    return Err(ModuleExecutionError::ExecutionFailed {
                        message: format!(
                            "Validation command failed: {}",
                            String::from_utf8_lossy(&output.stderr)
                        ),
                    });
                }

                results.insert(
                    "validation_output".to_string(),
                    serde_json::Value::String(String::from_utf8_lossy(&output.stdout).to_string()),
                );
            }
        }

        results.insert(
            "src".to_string(),
            serde_json::Value::String(args.src.clone()),
        );
        results.insert(
            "dest".to_string(),
            serde_json::Value::String(dest_path.to_string_lossy().to_string()),
        );

        Ok(ModuleResult {
            changed,
            failed: false,
            msg: Some("File copied successfully".to_string()),
            stdout: None,
            stderr: None,
            rc: Some(0),
            results,
            diff: None,
            warnings: vec![],
            ansible_facts: HashMap::new(),
        })
    }

    async fn analyze_copy_operation(
        &self,
        args: &CopyArgs,
        _context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        let src_path = Path::new(&args.src);
        let dest_path = Path::new(&args.dest);
        let mut results = HashMap::new();

        let src_exists = src_path.exists();
        let dest_exists = dest_path.exists();

        let would_change = if !src_exists {
            false // Can't copy non-existent file
        } else if !dest_exists {
            true // Would create new file
        } else {
            // Check if files are different
            self.files_are_different(src_path, dest_path)
                .await
                .unwrap_or(true)
        };

        results.insert(
            "src".to_string(),
            serde_json::Value::String(args.src.clone()),
        );
        results.insert(
            "dest".to_string(),
            serde_json::Value::String(args.dest.clone()),
        );
        results.insert(
            "src_exists".to_string(),
            serde_json::Value::Bool(src_exists),
        );
        results.insert(
            "dest_exists".to_string(),
            serde_json::Value::Bool(dest_exists),
        );
        results.insert(
            "would_change".to_string(),
            serde_json::Value::Bool(would_change),
        );

        Ok(ModuleResult {
            changed: false, // Never change in check mode
            failed: false,
            msg: Some("Check mode: no changes made".to_string()),
            stdout: None,
            stderr: None,
            rc: Some(0),
            results,
            diff: None,
            warnings: vec![],
            ansible_facts: HashMap::new(),
        })
    }

    async fn files_are_different(
        &self,
        src: &Path,
        dest: &Path,
    ) -> Result<bool, ModuleExecutionError> {
        if !src.exists() || !dest.exists() {
            return Ok(true);
        }

        // Quick size check first
        let src_metadata =
            fs::metadata(src)
                .await
                .map_err(|e| ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to get source metadata: {e}"),
                })?;

        let dest_metadata =
            fs::metadata(dest)
                .await
                .map_err(|e| ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to get destination metadata: {e}"),
                })?;

        if src_metadata.len() != dest_metadata.len() {
            return Ok(true);
        }

        // If sizes are the same, compare content
        let src_content =
            fs::read(src)
                .await
                .map_err(|e| ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to read source file: {e}"),
                })?;

        let dest_content =
            fs::read(dest)
                .await
                .map_err(|e| ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to read destination file: {e}"),
                })?;

        Ok(src_content != dest_content)
    }

    fn resolve_destination_path(
        &self,
        src_path: &Path,
        dest_path: &Path,
    ) -> Result<std::path::PathBuf, ModuleExecutionError> {
        if dest_path.is_dir() {
            // Copy into directory with source filename
            if let Some(filename) = src_path.file_name() {
                Ok(dest_path.join(filename))
            } else {
                Err(ModuleExecutionError::ExecutionFailed {
                    message: format!("Source path has no filename: {}", src_path.display()),
                })
            }
        } else {
            Ok(dest_path.to_path_buf())
        }
    }

    async fn copy_file(
        &self,
        src_path: &Path,
        dest_path: &Path,
        args: &CopyArgs,
    ) -> Result<bool, ModuleExecutionError> {
        // Check if destination exists and whether we should proceed
        let dest_exists = dest_path.exists();
        if dest_exists && !args.force.unwrap_or(false) {
            // Check if files are different
            let files_different = self.files_are_different(src_path, dest_path).await?;
            if !files_different {
                return Ok(false); // No changes needed
            }
        }

        // Create backup if requested and destination exists
        if args.backup.unwrap_or(false) && dest_exists {
            create_backup(dest_path, None).await.map_err(|e| {
                ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to create backup: {e}"),
                }
            })?;
        }

        // Create destination directory if it doesn't exist
        if let Some(parent_dir) = dest_path.parent() {
            if !parent_dir.exists() {
                fs::create_dir_all(parent_dir).await.map_err(|e| {
                    ModuleExecutionError::ExecutionFailed {
                        message: format!("Failed to create destination directory: {e}"),
                    }
                })?;

                // Set directory permissions if specified
                if let Some(dir_mode) = &args.directory_mode {
                    set_permissions(parent_dir, dir_mode).await.map_err(|e| {
                        ModuleExecutionError::ExecutionFailed {
                            message: format!("Failed to set directory permissions: {e}"),
                        }
                    })?;
                }
            }
        }

        // Perform atomic copy
        let mut writer = AtomicWriter::new(dest_path).await.map_err(|e| {
            ModuleExecutionError::ExecutionFailed {
                message: format!("Failed to create atomic writer: {e}"),
            }
        })?;

        let content =
            fs::read(src_path)
                .await
                .map_err(|e| ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to read source file: {e}"),
                })?;

        writer
            .write_all(&content)
            .await
            .map_err(|e| ModuleExecutionError::ExecutionFailed {
                message: format!("Failed to write destination file: {e}"),
            })?;

        writer
            .commit()
            .await
            .map_err(|e| ModuleExecutionError::ExecutionFailed {
                message: format!("Failed to commit file copy: {e}"),
            })?;

        // Set permissions - either preserve source or use specified mode
        if args.preserve.unwrap_or(false) {
            // Preserve source permissions
            let src_permissions = get_permissions(src_path).await.map_err(|e| {
                ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to get source permissions: {e}"),
                }
            })?;
            set_permissions(dest_path, &src_permissions)
                .await
                .map_err(|e| ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to preserve file permissions: {e}"),
                })?;
        } else if let Some(mode) = &args.mode {
            set_permissions(dest_path, mode).await.map_err(|e| {
                ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to set file permissions: {e}"),
                }
            })?;
        }

        // Set ownership if specified
        if args.owner.is_some() || args.group.is_some() {
            set_ownership(dest_path, args.owner.as_deref(), args.group.as_deref())
                .await
                .map_err(|e| ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to set file ownership: {e}"),
                })?;
        }

        Ok(true) // File was copied
    }

    async fn copy_directory(
        &self,
        src_path: &Path,
        dest_path: &Path,
        args: &CopyArgs,
    ) -> Result<bool, ModuleExecutionError> {
        let mut changed = false;

        // Create destination directory if it doesn't exist
        if !dest_path.exists() {
            fs::create_dir_all(dest_path).await.map_err(|e| {
                ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to create destination directory: {e}"),
                }
            })?;
            changed = true;
        }

        // Set directory permissions if specified
        if let Some(dir_mode) = &args.directory_mode {
            set_permissions(dest_path, dir_mode).await.map_err(|e| {
                ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to set directory permissions: {e}"),
                }
            })?;
        }

        // Recursively copy contents
        let mut entries =
            fs::read_dir(src_path)
                .await
                .map_err(|e| ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to read source directory: {e}"),
                })?;

        while let Some(entry) =
            entries
                .next_entry()
                .await
                .map_err(|e| ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to read directory entry: {e}"),
                })?
        {
            let entry_path = entry.path();
            let dest_entry_path = dest_path.join(entry.file_name());

            if entry_path.is_dir() {
                let result =
                    Box::pin(self.copy_directory(&entry_path, &dest_entry_path, args)).await?;
                if result {
                    changed = true;
                }
            } else if self.copy_file(&entry_path, &dest_entry_path, args).await? {
                changed = true;
            }
        }

        Ok(changed)
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
    async fn test_copy_file() {
        let temp_dir = TempDir::new().unwrap();
        let src_path = temp_dir.path().join("source.txt");
        let dest_path = temp_dir.path().join("destination.txt");

        // Create source file
        let mut src_file = tokio::fs::File::create(&src_path).await.unwrap();
        src_file.write_all(b"test content").await.unwrap();
        src_file.flush().await.unwrap();

        let args = ModuleArgs {
            args: {
                let mut map = HashMap::new();
                map.insert(
                    "src".to_string(),
                    serde_json::Value::String(src_path.to_string_lossy().to_string()),
                );
                map.insert(
                    "dest".to_string(),
                    serde_json::Value::String(dest_path.to_string_lossy().to_string()),
                );
                map
            },
            special: Default::default(),
        };

        let module = CopyModule;
        let context = create_test_context();
        let result = module.execute(&args, &context).await.unwrap();

        assert!(result.changed);
        assert!(dest_path.exists());

        let dest_content = tokio::fs::read_to_string(&dest_path).await.unwrap();
        assert_eq!(dest_content, "test content");
    }

    #[tokio::test]
    async fn test_copy_identical_files() {
        let temp_dir = TempDir::new().unwrap();
        let src_path = temp_dir.path().join("source.txt");
        let dest_path = temp_dir.path().join("destination.txt");

        // Create identical files
        let content = b"identical content";
        tokio::fs::write(&src_path, content).await.unwrap();
        tokio::fs::write(&dest_path, content).await.unwrap();

        let args = ModuleArgs {
            args: {
                let mut map = HashMap::new();
                map.insert(
                    "src".to_string(),
                    serde_json::Value::String(src_path.to_string_lossy().to_string()),
                );
                map.insert(
                    "dest".to_string(),
                    serde_json::Value::String(dest_path.to_string_lossy().to_string()),
                );
                map
            },
            special: Default::default(),
        };

        let module = CopyModule;
        let context = create_test_context();
        let result = module.execute(&args, &context).await.unwrap();

        assert!(!result.changed); // Files are identical, no change needed
    }
}
