//! Service module - manages system services

use async_trait::async_trait;
use std::collections::HashMap;

use crate::modules::{
    error::{ModuleExecutionError, ValidationError},
    interface::{
        ArgumentSpec, ExecutionContext, ExecutionModule, ModuleArgs, ModuleDocumentation,
        ModuleResult, Platform, ReturnValueSpec,
    },
    system::service_managers::ServiceManager,
};

/// Service module - manages system services
pub struct ServiceModule {
    service_managers: HashMap<Platform, Box<dyn ServiceManager>>,
}

impl Default for ServiceModule {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceModule {
    pub fn new() -> Self {
        let mut service_managers: HashMap<Platform, Box<dyn ServiceManager>> = HashMap::new();

        // Register platform-specific service managers
        #[cfg(target_os = "linux")]
        {
            use crate::modules::system::service_managers::{
                InitServiceManager, SystemdServiceManager,
            };
            service_managers.insert(Platform::Linux, Box::new(SystemdServiceManager::new()));
        }

        #[cfg(target_os = "macos")]
        {
            use crate::modules::system::service_managers::LaunchdServiceManager;
            service_managers.insert(Platform::MacOS, Box::new(LaunchdServiceManager::new()));
        }

        #[cfg(target_os = "windows")]
        {
            use crate::modules::system::service_managers::WindowsServiceManager;
            service_managers.insert(Platform::Windows, Box::new(WindowsServiceManager::new()));
        }

        Self { service_managers }
    }
}

#[async_trait]
impl ExecutionModule for ServiceModule {
    fn name(&self) -> &'static str {
        "service"
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

        let state = args.args.get("state").and_then(|v| v.as_str());
        let enabled = args.args.get("enabled").and_then(|v| v.as_bool());

        let service_manager = self
            .service_managers
            .get(&context.host_info.platform)
            .ok_or_else(|| {
                ModuleExecutionError::UnsupportedPlatform(context.host_info.platform.clone())
            })?;

        let current_status = service_manager.query_service(name).await.map_err(|e| {
            ModuleExecutionError::ExecutionFailed {
                message: e.to_string(),
            }
        })?;

        let mut changed = false;
        let mut actions = Vec::new();

        // Handle state changes
        if let Some(target_state) = state {
            match target_state {
                "started" | "running" => {
                    if !current_status.running {
                        changed = true;
                        actions.push("start".to_string());
                    }
                }
                "stopped" => {
                    if current_status.running {
                        changed = true;
                        actions.push("stop".to_string());
                    }
                }
                "restarted" | "reloaded" => {
                    // Always change for restart/reload
                    changed = true;
                    actions.push(target_state.to_string());
                }
                _ => {
                    return Err(ModuleExecutionError::InvalidArgs {
                        message: format!("Invalid state: {target_state}"),
                    })
                }
            }
        }

        // Handle enabled changes
        if let Some(target_enabled) = enabled {
            if current_status.enabled != Some(target_enabled) {
                changed = true;
                actions.push(if target_enabled { "enable" } else { "disable" }.to_string());
            }
        }

        if context.check_mode {
            return Ok(ModuleResult {
                changed,
                failed: false,
                msg: Some(format!("Service {name} would be modified: {actions:?}")),
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
                msg: Some(format!("Service {name} is already in desired state")),
                stdout: None,
                stderr: None,
                rc: Some(0),
                results: HashMap::new(),
                diff: None,
                warnings: Vec::new(),
                ansible_facts: HashMap::new(),
            });
        }

        // Execute actions
        for action in &actions {
            let result = match action.as_str() {
                "start" => service_manager.start_service(name).await,
                "stop" => service_manager.stop_service(name).await,
                "restarted" => service_manager.restart_service(name).await,
                "reloaded" => service_manager.reload_service(name).await,
                "enable" => service_manager.enable_service(name).await,
                "disable" => service_manager.disable_service(name).await,
                _ => continue,
            }
            .map_err(|e| ModuleExecutionError::ExecutionFailed {
                message: e.to_string(),
            })?;

            if !result.success {
                return Ok(ModuleResult {
                    changed: true,
                    failed: true,
                    msg: Some(format!(
                        "Failed to {} service {}: {}",
                        action, name, result.stderr
                    )),
                    stdout: Some(result.stdout),
                    stderr: Some(result.stderr),
                    rc: Some(result.exit_code),
                    results: HashMap::new(),
                    diff: None,
                    warnings: Vec::new(),
                    ansible_facts: HashMap::new(),
                });
            }
        }

        Ok(ModuleResult {
            changed: true,
            failed: false,
            msg: Some(format!("Service {name} successfully modified")),
            stdout: None,
            stderr: None,
            rc: Some(0),
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

        Ok(ModuleResult {
            changed: true, // Assume it would change for check mode
            failed: false,
            msg: Some(format!("Service {name} would be modified")),
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
            description: "Manage services".to_string(),
            arguments: vec![
                ArgumentSpec {
                    name: "name".to_string(),
                    description: "Name of the service".to_string(),
                    required: true,
                    argument_type: "str".to_string(),
                    default: None,
                },
                ArgumentSpec {
                    name: "state".to_string(),
                    description: "started/stopped are idempotent actions that will not run commands unless necessary. restarted will always bounce the service. reloaded will always reload the service.".to_string(),
                    required: false,
                    argument_type: "str".to_string(),
                    default: None,
                },
                ArgumentSpec {
                    name: "enabled".to_string(),
                    description: "Whether the service should start on boot".to_string(),
                    required: false,
                    argument_type: "bool".to_string(),
                    default: None,
                },
            ],
            examples: vec![
                r#"service:
    name: httpd
    state: started"#.to_string(),
                r#"service:
    name: httpd
    state: started
    enabled: yes"#.to_string(),
            ],
            return_values: vec![
                ReturnValueSpec {
                    name: "msg".to_string(),
                    description: "A short description of what happened".to_string(),
                    returned: "always".to_string(),
                    value_type: "str".to_string(),
                },
            ],
        }
    }
}
