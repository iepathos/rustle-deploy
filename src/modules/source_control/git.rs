//! Git module for version control operations

use crate::modules::{
    error::{ModuleExecutionError, ValidationError},
    interface::{ExecutionContext, ExecutionModule, ModuleArgs, ModuleResult, Platform},
    source_control::utils::{CredentialHandler, SshManager},
};
use async_trait::async_trait;
use git2::{
    build::RepoBuilder, BranchType, FetchOptions, RemoteCallbacks, Repository, RepositoryState,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::task;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitArgs {
    pub repo: String,
    pub dest: String,
    pub version: Option<String>,
    #[serde(default)]
    pub force: Option<bool>,
    pub depth: Option<u32>,
    #[serde(default)]
    pub clone: Option<bool>,
    #[serde(default)]
    pub update: Option<bool>,
    #[serde(default)]
    pub track_submodules: Option<bool>,
    pub key_file: Option<String>,
    #[serde(default)]
    pub accept_hostkey: Option<bool>,
    pub archive: Option<String>,
    pub separate_git_dir: Option<String>,
    #[serde(default)]
    pub verify_commit: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitResult {
    pub changed: bool,
    pub before: Option<String>,
    pub after: String,
    pub remote_url_changed: bool,
    pub warnings: Vec<String>,
}

pub struct GitModule {
    ssh_manager: SshManager,
}

impl GitModule {
    pub fn new() -> Self {
        Self {
            ssh_manager: SshManager::default(),
        }
    }

    async fn execute_git_operation(
        &self,
        args: &GitArgs,
        _context: &ExecutionContext,
    ) -> Result<GitResult, ModuleExecutionError> {
        let dest_path = Path::new(&args.dest);
        let repo_exists = dest_path.join(".git").exists();

        let mut warnings = Vec::new();

        // Check if we should validate SSH host key
        if let Some(hostname) = SshManager::extract_hostname(&args.repo) {
            if !args.accept_hostkey.unwrap_or(false) && !self.ssh_manager.is_known_host(&hostname) {
                warnings.push(format!(
                    "Host {} is not in known hosts. Consider setting accept_hostkey: true",
                    hostname
                ));
            }
        }

        let result = if repo_exists {
            if args.update.unwrap_or(true) {
                self.update_repository(args, dest_path, &mut warnings)
                    .await?
            } else {
                self.get_current_state(dest_path).await?
            }
        } else {
            if args.clone.unwrap_or(true) {
                self.clone_repository(args, dest_path, &mut warnings)
                    .await?
            } else {
                return Err(ModuleExecutionError::ExecutionFailed {
                    message: format!(
                        "Repository does not exist at {} and clone is disabled",
                        args.dest
                    ),
                });
            }
        };

        Ok(GitResult {
            changed: result.changed,
            before: result.before,
            after: result.after,
            remote_url_changed: result.remote_url_changed,
            warnings,
        })
    }

    async fn clone_repository(
        &self,
        args: &GitArgs,
        dest_path: &Path,
        warnings: &mut Vec<String>,
    ) -> Result<GitResult, ModuleExecutionError> {
        let args = args.clone();
        let dest_path = dest_path.to_path_buf();

        let result = task::spawn_blocking(move || Self::clone_repository_sync(&args, &dest_path))
            .await
            .map_err(|e| ModuleExecutionError::ExecutionFailed {
                message: format!("Task join error: {}", e),
            })??;

        warnings.extend(result.warnings.clone());
        Ok(result)
    }

    fn clone_repository_sync(
        args: &GitArgs,
        dest_path: &Path,
    ) -> Result<GitResult, ModuleExecutionError> {
        let mut warnings = Vec::new();
        // Set up credential handler
        let mut cred_handler = CredentialHandler::new();
        if let Some(key_file) = &args.key_file {
            cred_handler = cred_handler.with_ssh_key(key_file);

            // Validate SSH key
            if let Err(e) = cred_handler.validate_ssh_key() {
                warnings.push(format!("SSH key validation warning: {}", e));
            }
        }

        // Set up callbacks
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(move |_url, username_from_url, allowed_types| {
            cred_handler
                .get_credentials(username_from_url, allowed_types)
                .map_err(|e| git2::Error::from_str(&e.to_string()))
        });

        // Set up progress reporting
        callbacks.transfer_progress(|stats| {
            tracing::debug!(
                "Transfer progress: {}/{} objects, {} bytes",
                stats.received_objects(),
                stats.total_objects(),
                stats.received_bytes()
            );
            true
        });

        // Configure fetch options
        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        // Set up repository builder
        let mut builder = RepoBuilder::new();
        builder.fetch_options(fetch_options);

        if let Some(_depth) = args.depth {
            // Note: git2 doesn't directly support shallow clones with depth
            // This would need to be implemented with direct git commands
            warnings.push("Shallow clone depth not fully supported with git2".to_string());
        }

        if args.track_submodules.unwrap_or(false) {
            warnings.push("Submodule tracking not yet implemented".to_string());
        }

        // Perform the clone
        let repo = builder.clone(&args.repo, dest_path).map_err(|e| {
            ModuleExecutionError::ExecutionFailed {
                message: format!("Clone failed: {}", e),
            }
        })?;

        // Checkout specific version if requested
        let final_commit = if let Some(version) = &args.version {
            Self::checkout_version(&repo, version)?
        } else {
            Self::get_head_commit(&repo)?
        };

        Ok(GitResult {
            changed: true,
            before: None,
            after: final_commit,
            remote_url_changed: false,
            warnings,
        })
    }

    async fn update_repository(
        &self,
        args: &GitArgs,
        dest_path: &Path,
        warnings: &mut Vec<String>,
    ) -> Result<GitResult, ModuleExecutionError> {
        let args = args.clone();
        let dest_path = dest_path.to_path_buf();

        let result = task::spawn_blocking(move || Self::update_repository_sync(&args, &dest_path))
            .await
            .map_err(|e| ModuleExecutionError::ExecutionFailed {
                message: format!("Task join error: {}", e),
            })??;

        warnings.extend(result.warnings.clone());
        Ok(result)
    }

    fn update_repository_sync(
        args: &GitArgs,
        dest_path: &Path,
    ) -> Result<GitResult, ModuleExecutionError> {
        let mut warnings = Vec::new();
        let repo =
            Repository::open(dest_path).map_err(|e| ModuleExecutionError::ExecutionFailed {
                message: format!("Failed to open repository: {}", e),
            })?;

        // Check repository state
        let repo_state = repo.state();
        if repo_state != RepositoryState::Clean {
            if args.force.unwrap_or(false) {
                warnings.push(format!(
                    "Repository is in {:?} state, forcing update",
                    repo_state
                ));
                // Reset to clean state
                let head = repo.head()?;
                let commit = head.peel_to_commit()?;
                repo.reset(&commit.into_object(), git2::ResetType::Hard, None)?;
            } else {
                return Err(ModuleExecutionError::ExecutionFailed {
                    message: format!(
                        "Repository is in {:?} state. Use force: true to override",
                        repo_state
                    ),
                });
            }
        }

        let before_commit = Self::get_head_commit(&repo)?;

        // Set up credential handler
        let mut cred_handler = CredentialHandler::new();
        if let Some(key_file) = &args.key_file {
            cred_handler = cred_handler.with_ssh_key(key_file);
        }

        // Set up callbacks for fetch
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(move |_url, username_from_url, allowed_types| {
            cred_handler
                .get_credentials(username_from_url, allowed_types)
                .map_err(|e| git2::Error::from_str(&e.to_string()))
        });

        // Fetch from origin
        let mut remote =
            repo.find_remote("origin")
                .map_err(|e| ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to find origin remote: {}", e),
                })?;

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        remote
            .fetch(&[] as &[&str], Some(&mut fetch_options), None)
            .map_err(|e| ModuleExecutionError::ExecutionFailed {
                message: format!("Fetch failed: {}", e),
            })?;

        // Update to latest or specific version
        let after_commit = if let Some(version) = &args.version {
            Self::checkout_version(&repo, version)?
        } else {
            // Fast-forward merge to origin/main or origin/master
            Self::merge_fast_forward(&repo)?
        };

        let changed = before_commit != after_commit;

        Ok(GitResult {
            changed,
            before: Some(before_commit),
            after: after_commit,
            remote_url_changed: false,
            warnings,
        })
    }

    async fn get_current_state(&self, dest_path: &Path) -> Result<GitResult, ModuleExecutionError> {
        let dest_path = dest_path.to_path_buf();

        task::spawn_blocking(move || {
            let repo = Repository::open(&dest_path).map_err(|e| {
                ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to open repository: {}", e),
                }
            })?;

            let current_commit = Self::get_head_commit(&repo)?;

            Ok(GitResult {
                changed: false,
                before: Some(current_commit.clone()),
                after: current_commit,
                remote_url_changed: false,
                warnings: vec![],
            })
        })
        .await
        .map_err(|e| ModuleExecutionError::ExecutionFailed {
            message: format!("Task join error: {}", e),
        })?
    }

    fn checkout_version(repo: &Repository, version: &str) -> Result<String, ModuleExecutionError> {
        // Try to resolve as branch first
        if let Ok(branch) = repo.find_branch(version, BranchType::Local) {
            let commit = branch.get().peel_to_commit()?;
            repo.set_head(branch.get().name().unwrap())?;
            repo.checkout_head(None)?;
            return Ok(commit.id().to_string());
        }

        // Try remote branch
        if let Ok(branch) = repo.find_branch(&format!("origin/{}", version), BranchType::Remote) {
            let commit = branch.get().peel_to_commit()?;

            // Create local branch tracking remote
            let local_branch = repo.branch(version, &commit, false)?;
            let branch_ref = local_branch.get();

            repo.set_head(branch_ref.name().unwrap())?;
            repo.checkout_head(None)?;
            return Ok(commit.id().to_string());
        }

        // Try to resolve as tag
        if let Ok(tag_ref) = repo.find_reference(&format!("refs/tags/{}", version)) {
            let commit = tag_ref.peel_to_commit()?;
            repo.set_head_detached(commit.id())?;
            repo.checkout_head(None)?;
            return Ok(commit.id().to_string());
        }

        // Try to resolve as commit SHA
        if let Ok(oid) = git2::Oid::from_str(version) {
            if let Ok(commit) = repo.find_commit(oid) {
                repo.set_head_detached(commit.id())?;
                repo.checkout_head(None)?;
                return Ok(commit.id().to_string());
            }
        }

        Err(ModuleExecutionError::ExecutionFailed {
            message: format!("Could not resolve version: {}", version),
        })
    }

    fn merge_fast_forward(repo: &Repository) -> Result<String, ModuleExecutionError> {
        let head = repo.head()?;
        let head_commit = head.peel_to_commit()?;

        // Try to find origin/main or origin/master
        let upstream_names = ["origin/main", "origin/master"];

        for upstream_name in &upstream_names {
            if let Ok(upstream_ref) =
                repo.find_reference(&format!("refs/remotes/{}", upstream_name))
            {
                let upstream_commit = upstream_ref.peel_to_commit()?;

                // Check if we can fast-forward
                if repo.graph_descendant_of(upstream_commit.id(), head_commit.id())? {
                    // Already up to date
                    return Ok(head_commit.id().to_string());
                }

                if repo.graph_descendant_of(head_commit.id(), upstream_commit.id())? {
                    // Can fast-forward
                    let commit_id = upstream_commit.id().to_string();
                    repo.checkout_tree(&upstream_commit.into_object(), None)?;
                    repo.set_head(&format!("refs/remotes/{}", upstream_name))?;
                    return Ok(commit_id);
                }

                // Cannot fast-forward (diverged)
                return Err(ModuleExecutionError::ExecutionFailed {
                    message: "Repository has diverged from upstream. Use force: true to reset"
                        .to_string(),
                });
            }
        }

        // No upstream found, return current commit
        Ok(head_commit.id().to_string())
    }

    fn get_head_commit(repo: &Repository) -> Result<String, ModuleExecutionError> {
        let head = repo
            .head()
            .map_err(|e| ModuleExecutionError::ExecutionFailed {
                message: format!("Failed to get HEAD: {}", e),
            })?;

        let commit = head
            .peel_to_commit()
            .map_err(|e| ModuleExecutionError::ExecutionFailed {
                message: format!("Failed to peel HEAD to commit: {}", e),
            })?;

        Ok(commit.id().to_string())
    }
}

#[async_trait]
impl ExecutionModule for GitModule {
    fn name(&self) -> &'static str {
        "git"
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
            description: "Manage Git repositories - clone, update, checkout".to_string(),
            arguments: vec![
                ArgumentSpec {
                    name: "repo".to_string(),
                    description: "Git repository URL".to_string(),
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
            examples: vec!["git:
  repo: 'https://github.com/user/repo.git'
  dest: '/path/to/clone'"
                .to_string()],
            return_values: vec![ReturnValueSpec {
                name: "changed".to_string(),
                description: "Whether repository was modified".to_string(),
                returned: "always".to_string(),
                value_type: "boolean".to_string(),
            }],
        }
    }

    fn validate_args(&self, args: &ModuleArgs) -> Result<(), ValidationError> {
        let git_args: GitArgs =
            serde_json::from_value(serde_json::to_value(&args.args)?).map_err(|e| {
                ValidationError::InvalidArgValue {
                    arg: "args".to_string(),
                    value: "<complex>".to_string(),
                    reason: e.to_string(),
                }
            })?;

        if git_args.repo.is_empty() {
            return Err(ValidationError::MissingRequiredArg {
                arg: "repo".to_string(),
            });
        }

        if git_args.dest.is_empty() {
            return Err(ValidationError::MissingRequiredArg {
                arg: "dest".to_string(),
            });
        }

        // Validate URL format
        if !git_args.repo.starts_with("https://")
            && !git_args.repo.starts_with("http://")
            && !git_args.repo.starts_with("git@")
            && !git_args.repo.starts_with("ssh://")
        {
            return Err(ValidationError::InvalidArgValue {
                arg: "repo".to_string(),
                value: git_args.repo.clone(),
                reason: "must be a valid Git URL".to_string(),
            });
        }

        // Validate depth if provided
        if let Some(depth) = git_args.depth {
            if depth == 0 {
                return Err(ValidationError::InvalidArgValue {
                    arg: "depth".to_string(),
                    value: depth.to_string(),
                    reason: "must be greater than 0".to_string(),
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
        let git_args: GitArgs =
            serde_json::from_value(serde_json::to_value(&args.args)?).map_err(|e| {
                ModuleExecutionError::InvalidArgs {
                    message: e.to_string(),
                }
            })?;

        let result = self.execute_git_operation(&git_args, context).await?;

        let msg = if result.changed {
            format!(
                "Repository {} updated to commit {}",
                git_args.repo,
                &result.after[..8]
            )
        } else {
            format!(
                "Repository {} already at commit {}",
                git_args.repo,
                &result.after[..8]
            )
        };

        let mut results = HashMap::new();
        results.insert(
            "git_result".to_string(),
            serde_json::to_value(result.clone()).unwrap(),
        );

        Ok(ModuleResult {
            changed: result.changed,
            failed: false,
            msg: Some(msg),
            stdout: None,
            stderr: None,
            rc: Some(0),
            results,
            diff: None,
            warnings: result.warnings,
            ansible_facts: HashMap::new(),
        })
    }

    async fn check_mode(
        &self,
        args: &ModuleArgs,
        _context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        let git_args: GitArgs =
            serde_json::from_value(serde_json::to_value(&args.args)?).map_err(|e| {
                ModuleExecutionError::InvalidArgs {
                    message: e.to_string(),
                }
            })?;

        let dest_path = Path::new(&git_args.dest);
        let repo_exists = dest_path.join(".git").exists();

        let (would_change, message) = if repo_exists {
            if git_args.update.unwrap_or(true) {
                (
                    true,
                    format!("Would update repository at {}", git_args.dest),
                )
            } else {
                (
                    false,
                    format!("Would check repository status at {}", git_args.dest),
                )
            }
        } else {
            if git_args.clone.unwrap_or(true) {
                (
                    true,
                    format!("Would clone {} to {}", git_args.repo, git_args.dest),
                )
            } else {
                (
                    false,
                    format!("Repository does not exist and clone is disabled"),
                )
            }
        };

        Ok(ModuleResult {
            changed: would_change,
            failed: false,
            msg: Some(message),
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

impl Default for GitModule {
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
        let module = GitModule::new();

        // Test valid args
        let valid_args_json = serde_json::json!({
            "repo": "https://github.com/user/repo.git",
            "dest": "/path/to/dest"
        });
        let valid_args = ModuleArgs {
            args: serde_json::from_value(valid_args_json).unwrap(),
            special: crate::modules::interface::SpecialParameters::default(),
        };
        assert!(module.validate_args(&valid_args).is_ok());

        // Test missing repo
        let invalid_args_json = serde_json::json!({
            "dest": "/path/to/dest"
        });
        let invalid_args = ModuleArgs {
            args: serde_json::from_value(invalid_args_json).unwrap(),
            special: crate::modules::interface::SpecialParameters::default(),
        };
        assert!(module.validate_args(&invalid_args).is_err());

        // Test invalid URL
        let invalid_args_json = serde_json::json!({
            "repo": "not-a-url",
            "dest": "/path/to/dest"
        });
        let invalid_args = ModuleArgs {
            args: serde_json::from_value(invalid_args_json).unwrap(),
            special: crate::modules::interface::SpecialParameters::default(),
        };
        assert!(module.validate_args(&invalid_args).is_err());

        // Test zero depth
        let invalid_args_json = serde_json::json!({
            "repo": "https://github.com/user/repo.git",
            "dest": "/path/to/dest",
            "depth": 0
        });
        let invalid_args = ModuleArgs {
            args: serde_json::from_value(invalid_args_json).unwrap(),
            special: crate::modules::interface::SpecialParameters::default(),
        };
        assert!(module.validate_args(&invalid_args).is_err());
    }

    #[test]
    fn test_git_args_deserialization() {
        let json = serde_json::json!({
            "repo": "https://github.com/user/repo.git",
            "dest": "/path/to/dest",
            "version": "main",
            "depth": 1,
            "force": true
        });

        let args: GitArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.repo, "https://github.com/user/repo.git");
        assert_eq!(args.dest, "/path/to/dest");
        assert_eq!(args.version, Some("main".to_string()));
        assert_eq!(args.depth, Some(1));
        assert_eq!(args.force, Some(true));
    }
}
