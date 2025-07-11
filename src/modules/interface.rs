//! Module interface traits and types

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::modules::error::{ModuleExecutionError, ValidationError};

/// Unified interface for all execution modules
#[async_trait]
pub trait ExecutionModule: Send + Sync {
    /// Module name (e.g., "command", "package", "debug")
    fn name(&self) -> &'static str;

    /// Module version
    fn version(&self) -> &'static str;

    /// Supported platforms
    fn supported_platforms(&self) -> &[Platform];

    /// Execute the module with given arguments
    async fn execute(
        &self,
        args: &ModuleArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError>;

    /// Validate module arguments before execution
    fn validate_args(&self, args: &ModuleArgs) -> Result<(), ValidationError>;

    /// Check if module operation would make changes (dry-run)
    async fn check_mode(
        &self,
        args: &ModuleArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError>;

    /// Get module documentation
    fn documentation(&self) -> ModuleDocumentation;
}

/// Module execution arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleArgs {
    /// Direct module arguments
    pub args: HashMap<String, serde_json::Value>,
    /// Special parameters
    pub special: SpecialParameters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialParameters {
    pub r#become: Option<BecomeConfig>,
    pub when: Option<String>,
    pub changed_when: Option<String>,
    pub failed_when: Option<String>,
    pub check_mode: bool,
    pub diff: bool,
}

impl Default for SpecialParameters {
    fn default() -> Self {
        Self {
            r#become: None,
            when: None,
            changed_when: None,
            failed_when: None,
            check_mode: false,
            diff: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BecomeConfig {
    pub method: String, // sudo, su, runas, etc.
    pub user: String,
    pub password: Option<String>,
    pub flags: Vec<String>,
}

/// Module execution context
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub facts: HashMap<String, serde_json::Value>,
    pub variables: HashMap<String, serde_json::Value>,
    pub host_info: HostInfo,
    pub working_directory: PathBuf,
    pub environment: HashMap<String, String>,
    pub check_mode: bool,
    pub diff_mode: bool,
    pub verbosity: u8,
}

#[derive(Debug, Clone)]
pub struct HostInfo {
    pub hostname: String,
    pub platform: Platform,
    pub architecture: String,
    pub os_family: String,
    pub distribution: Option<String>,
    pub distribution_version: Option<String>,
}

impl HostInfo {
    pub fn detect() -> Self {
        let hostname = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown".to_string());

        let platform = if cfg!(target_os = "linux") {
            Platform::Linux
        } else if cfg!(target_os = "macos") {
            Platform::MacOS
        } else if cfg!(target_os = "windows") {
            Platform::Windows
        } else if cfg!(target_os = "freebsd") {
            Platform::FreeBSD
        } else if cfg!(target_os = "openbsd") {
            Platform::OpenBSD
        } else if cfg!(target_os = "netbsd") {
            Platform::NetBSD
        } else {
            Platform::Linux // Default fallback
        };

        let architecture = std::env::consts::ARCH.to_string();
        let os_family = std::env::consts::FAMILY.to_string();

        Self {
            hostname,
            platform,
            architecture,
            os_family,
            distribution: None, // Could be detected later
            distribution_version: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Platform {
    Linux,
    MacOS,
    Windows,
    FreeBSD,
    OpenBSD,
    NetBSD,
}

/// Module execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleResult {
    pub changed: bool,
    pub failed: bool,
    pub msg: Option<String>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub rc: Option<i32>,
    pub results: HashMap<String, serde_json::Value>,
    pub diff: Option<Diff>,
    pub warnings: Vec<String>,
    pub ansible_facts: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diff {
    pub before: Option<String>,
    pub after: Option<String>,
    pub before_header: Option<String>,
    pub after_header: Option<String>,
}

/// Module documentation
#[derive(Debug, Clone)]
pub struct ModuleDocumentation {
    pub description: String,
    pub arguments: Vec<ArgumentSpec>,
    pub examples: Vec<String>,
    pub return_values: Vec<ReturnValueSpec>,
}

#[derive(Debug, Clone)]
pub struct ArgumentSpec {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub argument_type: String,
    pub default: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReturnValueSpec {
    pub name: String,
    pub description: String,
    pub returned: String,
    pub value_type: String,
}
