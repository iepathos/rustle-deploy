//! File module for managing file attributes and state

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

use super::platform;
use super::utils::{
    backup::create_simple_backup,
    ownership::{get_ownership, set_ownership},
    permissions::{get_permissions, set_permissions},
};

/// File state options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileState {
    Present,   // Ensure file exists
    Absent,    // Ensure file doesn't exist
    Directory, // Ensure directory exists
    Link,      // Create symbolic link
    Hard,      // Create hard link
    Touch,     // Touch file (update timestamp)
}

impl std::str::FromStr for FileState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "present" => Ok(FileState::Present),
            "absent" => Ok(FileState::Absent),
            "directory" => Ok(FileState::Directory),
            "link" => Ok(FileState::Link),
            "hard" => Ok(FileState::Hard),
            "touch" => Ok(FileState::Touch),
            _ => Err(format!("Invalid file state: {s}")),
        }
    }
}

/// File module arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileArgs {
    pub path: String,             // Required: target file/directory path
    pub state: Option<FileState>, // File state (default: present)
    pub mode: Option<String>,     // File permissions (0644, u+rwx, etc.)
    pub owner: Option<String>,    // File owner (username or UID)
    pub group: Option<String>,    // File group (groupname or GID)
    pub src: Option<String>,      // Source for link operations
    pub recurse: Option<bool>,    // Recursive operations for directories
    pub follow: Option<bool>,     // Follow symlinks
    pub force: Option<bool>,      // Force operations
    pub backup: Option<bool>,     // Create backup before changes
}

impl FileArgs {
    pub fn from_module_args(args: &ModuleArgs) -> Result<Self, ValidationError> {
        let mut file_args = Self {
            path: String::new(),
            state: None,
            mode: None,
            owner: None,
            group: None,
            src: None,
            recurse: None,
            follow: None,
            force: None,
            backup: None,
        };

        // Required path
        if let Some(path) = args.args.get("path") {
            file_args.path = path
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
        if let Some(state) = args.args.get("state") {
            if let Some(state_str) = state.as_str() {
                file_args.state =
                    Some(
                        state_str
                            .parse()
                            .map_err(|e| ValidationError::InvalidArgValue {
                                arg: "state".to_string(),
                                value: state_str.to_string(),
                                reason: e,
                            })?,
                    );
            }
        }

        if let Some(mode) = args.args.get("mode") {
            file_args.mode = mode.as_str().map(|s| s.to_string());
        }

        if let Some(owner) = args.args.get("owner") {
            file_args.owner = owner.as_str().map(|s| s.to_string());
        }

        if let Some(group) = args.args.get("group") {
            file_args.group = group.as_str().map(|s| s.to_string());
        }

        if let Some(src) = args.args.get("src") {
            file_args.src = src.as_str().map(|s| s.to_string());
        }

        if let Some(recurse) = args.args.get("recurse") {
            file_args.recurse = recurse.as_bool();
        }

        if let Some(follow) = args.args.get("follow") {
            file_args.follow = follow.as_bool();
        }

        if let Some(force) = args.args.get("force") {
            file_args.force = force.as_bool();
        }

        if let Some(backup) = args.args.get("backup") {
            file_args.backup = backup.as_bool();
        }

        Ok(file_args)
    }
}

/// File module implementation
pub struct FileModule;

#[async_trait]
impl ExecutionModule for FileModule {
    fn name(&self) -> &'static str {
        "file"
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
        let file_args =
            FileArgs::from_module_args(args).map_err(ModuleExecutionError::Validation)?;

        self.execute_file_operation(&file_args, context).await
    }

    fn validate_args(&self, args: &ModuleArgs) -> Result<(), ValidationError> {
        FileArgs::from_module_args(args)?;
        Ok(())
    }

    async fn check_mode(
        &self,
        args: &ModuleArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        let file_args =
            FileArgs::from_module_args(args).map_err(ModuleExecutionError::Validation)?;

        // In check mode, we analyze what would be done without making changes
        self.analyze_file_operation(&file_args, context).await
    }

    fn documentation(&self) -> ModuleDocumentation {
        ModuleDocumentation {
            description: "Manage file and directory attributes".to_string(),
            arguments: vec![
                ArgumentSpec {
                    name: "path".to_string(),
                    description: "Path to the file or directory".to_string(),
                    required: true,
                    argument_type: "str".to_string(),
                    default: None,
                },
                ArgumentSpec {
                    name: "state".to_string(),
                    description: "Desired state (present, absent, directory, link, hard, touch)"
                        .to_string(),
                    required: false,
                    argument_type: "str".to_string(),
                    default: Some("present".to_string()),
                },
                ArgumentSpec {
                    name: "mode".to_string(),
                    description: "File permissions (e.g., '0644', 'u+rwx')".to_string(),
                    required: false,
                    argument_type: "str".to_string(),
                    default: None,
                },
                ArgumentSpec {
                    name: "owner".to_string(),
                    description: "File owner (username or UID)".to_string(),
                    required: false,
                    argument_type: "str".to_string(),
                    default: None,
                },
                ArgumentSpec {
                    name: "group".to_string(),
                    description: "File group (groupname or GID)".to_string(),
                    required: false,
                    argument_type: "str".to_string(),
                    default: None,
                },
                ArgumentSpec {
                    name: "src".to_string(),
                    description: "Source file for link operations".to_string(),
                    required: false,
                    argument_type: "str".to_string(),
                    default: None,
                },
                ArgumentSpec {
                    name: "backup".to_string(),
                    description: "Create backup before making changes".to_string(),
                    required: false,
                    argument_type: "bool".to_string(),
                    default: Some("false".to_string()),
                },
            ],
            examples: vec![
                r#"file:
  path: /etc/myapp/config.conf
  mode: '0644'
  owner: root
  group: root"#
                    .to_string(),
                r#"file:
  path: /usr/bin/myapp
  src: /usr/local/bin/myapp
  state: link"#
                    .to_string(),
            ],
            return_values: vec![
                ReturnValueSpec {
                    name: "changed".to_string(),
                    description: "Whether the file was changed".to_string(),
                    returned: "always".to_string(),
                    value_type: "bool".to_string(),
                },
                ReturnValueSpec {
                    name: "path".to_string(),
                    description: "Path to the file".to_string(),
                    returned: "always".to_string(),
                    value_type: "str".to_string(),
                },
            ],
        }
    }
}

impl FileModule {
    async fn execute_file_operation(
        &self,
        args: &FileArgs,
        _context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        let path = Path::new(&args.path);
        let mut changed = false;
        let mut results = HashMap::new();
        let state = args.state.as_ref().unwrap_or(&FileState::Present);

        // Create backup if requested and file exists
        if args.backup.unwrap_or(false) && path.exists() {
            if let Ok(Some(backup_path)) = create_simple_backup(path).await {
                results.insert(
                    "backup_file".to_string(),
                    serde_json::Value::String(backup_path.display().to_string()),
                );
            }
        }

        match state {
            FileState::Present => {
                if !path.exists() {
                    fs::File::create(path).await.map_err(|e| {
                        ModuleExecutionError::ExecutionFailed {
                            message: format!("Failed to create file: {e}"),
                        }
                    })?;
                    changed = true;
                }
            }
            FileState::Absent => {
                if path.exists() {
                    if path.is_dir() {
                        if args.recurse.unwrap_or(false) {
                            fs::remove_dir_all(path).await
                        } else {
                            fs::remove_dir(path).await
                        }
                    } else {
                        fs::remove_file(path).await
                    }
                    .map_err(|e| ModuleExecutionError::ExecutionFailed {
                        message: format!("Failed to remove: {e}"),
                    })?;
                    changed = true;
                }
            }
            FileState::Directory => {
                if !path.exists() {
                    fs::create_dir_all(path).await.map_err(|e| {
                        ModuleExecutionError::ExecutionFailed {
                            message: format!("Failed to create directory: {e}"),
                        }
                    })?;
                    changed = true;
                } else if !path.is_dir() {
                    return Err(ModuleExecutionError::ExecutionFailed {
                        message: "Path exists but is not a directory".to_string(),
                    });
                }
            }
            FileState::Link => {
                let src =
                    args.src
                        .as_ref()
                        .ok_or_else(|| ModuleExecutionError::ExecutionFailed {
                            message: "src parameter required for link state".to_string(),
                        })?;

                if path.exists() && args.force.unwrap_or(false) {
                    fs::remove_file(path).await.ok();
                }

                if !path.exists() {
                    platform::create_symlink(Path::new(src), path)
                        .await
                        .map_err(|e| ModuleExecutionError::ExecutionFailed {
                            message: format!("Failed to create symlink: {e}"),
                        })?;
                    changed = true;
                }
            }
            FileState::Hard => {
                let src =
                    args.src
                        .as_ref()
                        .ok_or_else(|| ModuleExecutionError::ExecutionFailed {
                            message: "src parameter required for hard state".to_string(),
                        })?;

                if path.exists() && args.force.unwrap_or(false) {
                    fs::remove_file(path).await.ok();
                }

                if !path.exists() {
                    platform::create_hardlink(Path::new(src), path)
                        .await
                        .map_err(|e| ModuleExecutionError::ExecutionFailed {
                            message: format!("Failed to create hardlink: {e}"),
                        })?;
                    changed = true;
                }
            }
            FileState::Touch => {
                if !path.exists() {
                    fs::File::create(path).await.map_err(|e| {
                        ModuleExecutionError::ExecutionFailed {
                            message: format!("Failed to create file: {e}"),
                        }
                    })?;
                    changed = true;
                } else {
                    // Update timestamps
                    let now = std::time::SystemTime::now();
                    let file_time = filetime::FileTime::from_system_time(now);
                    filetime::set_file_times(path, file_time, file_time).map_err(|e| {
                        ModuleExecutionError::ExecutionFailed {
                            message: format!("Failed to set file times: {e}"),
                        }
                    })?;
                    changed = true;
                }
            }
        }

        // Set permissions if specified
        if let Some(mode) = &args.mode {
            if path.exists() {
                set_permissions(path, mode).await.map_err(|e| {
                    ModuleExecutionError::ExecutionFailed {
                        message: format!("Failed to set permissions: {e}"),
                    }
                })?;
                changed = true;
            }
        }

        // Set ownership if specified
        if (args.owner.is_some() || args.group.is_some()) && path.exists() {
            set_ownership(path, args.owner.as_deref(), args.group.as_deref())
                .await
                .map_err(|e| ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to set ownership: {e}"),
                })?;
            changed = true;
        }

        // Add file information to results
        if path.exists() {
            if let Ok(mode) = get_permissions(path).await {
                results.insert("mode".to_string(), serde_json::Value::String(mode));
            }
            if let Ok((owner, group)) = get_ownership(path).await {
                results.insert("owner".to_string(), serde_json::Value::String(owner));
                results.insert("group".to_string(), serde_json::Value::String(group));
            }
        }

        results.insert(
            "path".to_string(),
            serde_json::Value::String(args.path.clone()),
        );

        Ok(ModuleResult {
            changed,
            failed: false,
            msg: None,
            stdout: None,
            stderr: None,
            rc: Some(0),
            results,
            diff: None,
            warnings: vec![],
            ansible_facts: HashMap::new(),
        })
    }

    async fn analyze_file_operation(
        &self,
        args: &FileArgs,
        _context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        let path = Path::new(&args.path);
        let mut results = HashMap::new();
        let state = args.state.as_ref().unwrap_or(&FileState::Present);

        // Analyze what would be changed
        let would_change = match state {
            FileState::Present => !path.exists(),
            FileState::Absent => path.exists(),
            FileState::Directory => !path.exists() || !path.is_dir(),
            FileState::Link | FileState::Hard => !path.exists(),
            FileState::Touch => true, // Touch always updates timestamps
        };

        results.insert(
            "path".to_string(),
            serde_json::Value::String(args.path.clone()),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::interface::HostInfo;
    use std::path::PathBuf;
    use tempfile::TempDir;

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
    async fn test_file_create() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file.txt");

        let args = ModuleArgs {
            args: {
                let mut map = HashMap::new();
                map.insert(
                    "path".to_string(),
                    serde_json::Value::String(file_path.to_string_lossy().to_string()),
                );
                map.insert(
                    "state".to_string(),
                    serde_json::Value::String("present".to_string()),
                );
                map
            },
            special: Default::default(),
        };

        let module = FileModule;
        let context = create_test_context();
        let result = module.execute(&args, &context).await.unwrap();

        assert!(result.changed);
        assert!(file_path.exists());
    }

    #[tokio::test]
    async fn test_directory_create() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("test_dir");

        let args = ModuleArgs {
            args: {
                let mut map = HashMap::new();
                map.insert(
                    "path".to_string(),
                    serde_json::Value::String(dir_path.to_string_lossy().to_string()),
                );
                map.insert(
                    "state".to_string(),
                    serde_json::Value::String("directory".to_string()),
                );
                map
            },
            special: Default::default(),
        };

        let module = FileModule;
        let context = create_test_context();
        let result = module.execute(&args, &context).await.unwrap();

        assert!(result.changed);
        assert!(dir_path.exists());
        assert!(dir_path.is_dir());
    }
}
