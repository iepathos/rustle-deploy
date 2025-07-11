//! Package module - manages system packages

use async_trait::async_trait;
use std::collections::HashMap;

use crate::modules::{
    error::{ModuleExecutionError, ValidationError},
    interface::{
        ArgumentSpec, ExecutionContext, ExecutionModule, ModuleArgs, ModuleDocumentation,
        ModuleResult, Platform, ReturnValueSpec,
    },
    system::package_managers::{PackageManager, PackageState},
};

/// Package module - manages system packages
pub struct PackageModule {
    package_managers: HashMap<Platform, Box<dyn PackageManager>>,
}

impl Default for PackageModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PackageModule {
    pub fn new() -> Self {
        let mut package_managers: HashMap<Platform, Box<dyn PackageManager>> = HashMap::new();

        // Register platform-specific package managers
        #[cfg(target_os = "linux")]
        {
            use crate::modules::system::package_managers::{AptPackageManager, YumPackageManager};
            package_managers.insert(Platform::Linux, Box::new(AptPackageManager::new()));
        }

        #[cfg(target_os = "macos")]
        {
            use crate::modules::system::package_managers::BrewPackageManager;
            package_managers.insert(Platform::MacOS, Box::new(BrewPackageManager::new()));
        }

        #[cfg(target_os = "windows")]
        {
            use crate::modules::system::package_managers::ChocolateyPackageManager;
            package_managers.insert(Platform::Windows, Box::new(ChocolateyPackageManager::new()));
        }

        Self { package_managers }
    }
}

#[async_trait]
impl ExecutionModule for PackageModule {
    fn name(&self) -> &'static str {
        "package"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn supported_platforms(&self) -> &[Platform] {
        &[Platform::Linux, Platform::MacOS, Platform::Windows]
    }

    async fn execute(
        &self,
        args: &ModuleArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        let name = args
            .args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ModuleExecutionError::InvalidArgs {
                message: "name is required".to_string(),
            })?;

        let state = args
            .args
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("present");

        let package_manager = self
            .package_managers
            .get(&context.host_info.platform)
            .ok_or_else(|| {
                ModuleExecutionError::UnsupportedPlatform(context.host_info.platform.clone())
            })?;

        let current_state = package_manager.query_package(name).await.map_err(|e| {
            ModuleExecutionError::ExecutionFailed {
                message: e.to_string(),
            }
        })?;

        let target_state = match state {
            "present" | "installed" | "latest" => PackageState::Present,
            "absent" | "removed" => PackageState::Absent,
            _ => {
                return Err(ModuleExecutionError::InvalidArgs {
                    message: format!("Invalid state: {state}"),
                })
            }
        };

        let changed = !matches!(
            (current_state, target_state),
            (PackageState::Present, PackageState::Present)
                | (PackageState::Absent, PackageState::Absent)
        );

        if context.check_mode {
            return Ok(ModuleResult {
                changed,
                failed: false,
                msg: Some(format!("Package {name} would be {state}")),
                stdout: None,
                stderr: None,
                rc: None,
                results: HashMap::new(),
                diff: None,
                warnings: Vec::new(),
                ansible_facts: HashMap::new(),
            });
        }

        if !changed {
            return Ok(ModuleResult {
                changed: false,
                failed: false,
                msg: Some(format!("Package {name} is already {state}")),
                stdout: None,
                stderr: None,
                rc: Some(0),
                results: HashMap::new(),
                diff: None,
                warnings: Vec::new(),
                ansible_facts: HashMap::new(),
            });
        }

        let result = match target_state {
            PackageState::Present => package_manager.install_package(name).await.map_err(|e| {
                ModuleExecutionError::ExecutionFailed {
                    message: e.to_string(),
                }
            })?,
            PackageState::Absent => package_manager.remove_package(name).await.map_err(|e| {
                ModuleExecutionError::ExecutionFailed {
                    message: e.to_string(),
                }
            })?,
        };

        Ok(ModuleResult {
            changed: true,
            failed: !result.success,
            msg: result.message,
            stdout: Some(result.stdout),
            stderr: Some(result.stderr),
            rc: Some(result.exit_code),
            results: HashMap::new(),
            diff: None,
            warnings: Vec::new(),
            ansible_facts: HashMap::new(),
        })
    }

    fn validate_args(&self, args: &ModuleArgs) -> Result<(), ValidationError> {
        if !args.args.contains_key("name") {
            return Err(ValidationError::MissingRequiredArg {
                arg: "name".to_string(),
            });
        }
        Ok(())
    }

    async fn check_mode(
        &self,
        args: &ModuleArgs,
        _context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        let name = args
            .args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ModuleExecutionError::InvalidArgs {
                message: "name is required".to_string(),
            })?;

        let state = args
            .args
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("present");

        Ok(ModuleResult {
            changed: true, // Assume it would change for check mode
            failed: false,
            msg: Some(format!("Package {name} would be {state}")),
            stdout: None,
            stderr: None,
            rc: None,
            results: HashMap::new(),
            diff: None,
            warnings: Vec::new(),
            ansible_facts: HashMap::new(),
        })
    }

    fn documentation(&self) -> ModuleDocumentation {
        ModuleDocumentation {
            description: "Manage packages with the OS package manager".to_string(),
            arguments: vec![
                ArgumentSpec {
                    name: "name".to_string(),
                    description: "Package name or list of packages to install".to_string(),
                    required: true,
                    argument_type: "str or list".to_string(),
                    default: None,
                },
                ArgumentSpec {
                    name: "state".to_string(),
                    description: "Whether to install (present), or remove (absent) a package"
                        .to_string(),
                    required: false,
                    argument_type: "str".to_string(),
                    default: Some("present".to_string()),
                },
            ],
            examples: vec![
                r#"package:
    name: git
    state: present"#
                    .to_string(),
                r#"package:
    name: 
      - git
      - vim
    state: present"#
                    .to_string(),
            ],
            return_values: vec![ReturnValueSpec {
                name: "msg".to_string(),
                description: "A short description of what happened".to_string(),
                returned: "always".to_string(),
                value_type: "str".to_string(),
            }],
        }
    }
}
