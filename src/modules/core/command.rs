//! Command module - executes shell commands

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use tokio::process::Command;

use crate::modules::{
    error::{ModuleExecutionError, ValidationError},
    interface::{
        ArgumentSpec, ExecutionContext, ExecutionModule, ModuleArgs, ModuleDocumentation,
        ModuleResult, Platform, ReturnValueSpec,
    },
};

/// Command module - executes shell commands
pub struct CommandModule;

#[async_trait]
impl ExecutionModule for CommandModule {
    fn name(&self) -> &'static str {
        "command"
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
        let command = self.extract_command(args)?;
        let chdir = args.args.get("chdir").and_then(|v| v.as_str());
        let creates = args.args.get("creates").and_then(|v| v.as_str());
        let removes = args.args.get("removes").and_then(|v| v.as_str());

        // Check creates/removes conditions
        if let Some(creates_path) = creates {
            if Path::new(creates_path).exists() {
                return Ok(ModuleResult {
                    changed: false,
                    failed: false,
                    msg: Some(format!("{creates_path} already exists")),
                    stdout: None,
                    stderr: None,
                    rc: Some(0),
                    results: HashMap::new(),
                    diff: None,
                    warnings: Vec::new(),
                    ansible_facts: HashMap::new(),
                });
            }
        }

        if let Some(removes_path) = removes {
            if !Path::new(removes_path).exists() {
                return Ok(ModuleResult {
                    changed: false,
                    failed: false,
                    msg: Some(format!("{removes_path} does not exist")),
                    stdout: None,
                    stderr: None,
                    rc: Some(0),
                    results: HashMap::new(),
                    diff: None,
                    warnings: Vec::new(),
                    ansible_facts: HashMap::new(),
                });
            }
        }

        if context.check_mode {
            return Ok(ModuleResult {
                changed: true,
                failed: false,
                msg: Some("Command would run".to_string()),
                stdout: None,
                stderr: None,
                rc: None,
                results: HashMap::new(),
                diff: None,
                warnings: Vec::new(),
                ansible_facts: HashMap::new(),
            });
        }

        let mut cmd = self.build_command(&command, context)?;

        if let Some(dir) = chdir {
            cmd.current_dir(dir);
        }

        let output = cmd.output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let rc = output.status.code().unwrap_or(-1);

        Ok(ModuleResult {
            changed: true,
            failed: !output.status.success(),
            msg: if output.status.success() {
                None
            } else {
                Some(stderr.clone())
            },
            stdout: Some(stdout),
            stderr: Some(stderr),
            rc: Some(rc),
            results: HashMap::new(),
            diff: None,
            warnings: Vec::new(),
            ansible_facts: HashMap::new(),
        })
    }

    fn validate_args(&self, args: &ModuleArgs) -> Result<(), ValidationError> {
        if !args.args.contains_key("_raw_params") && !args.args.contains_key("cmd") {
            return Err(ValidationError::MissingRequiredArg {
                arg: "_raw_params or cmd".to_string(),
            });
        }
        Ok(())
    }

    async fn check_mode(
        &self,
        args: &ModuleArgs,
        _context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        // Command module can't safely run in check mode, so just indicate it would change
        let _command = self.extract_command(args)?; // Validate command exists

        Ok(ModuleResult {
            changed: true,
            failed: false,
            msg: Some("Command would run".to_string()),
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
            description: "Execute commands on targets".to_string(),
            arguments: vec![
                ArgumentSpec {
                    name: "_raw_params".to_string(),
                    description: "The command module takes a free form command to run. There is no actual parameter named '_raw_params'.".to_string(),
                    required: true,
                    argument_type: "str".to_string(),
                    default: None,
                },
                ArgumentSpec {
                    name: "cmd".to_string(),
                    description: "The command to run. Alternative to _raw_params.".to_string(),
                    required: false,
                    argument_type: "str or list".to_string(),
                    default: None,
                },
                ArgumentSpec {
                    name: "chdir".to_string(),
                    description: "Change into this directory before running the command.".to_string(),
                    required: false,
                    argument_type: "path".to_string(),
                    default: None,
                },
                ArgumentSpec {
                    name: "creates".to_string(),
                    description: "A filename or glob pattern. If it already exists, this step won't be run.".to_string(),
                    required: false,
                    argument_type: "path".to_string(),
                    default: None,
                },
                ArgumentSpec {
                    name: "removes".to_string(),
                    description: "A filename or glob pattern. If it doesn't exist, this step won't be run.".to_string(),
                    required: false,
                    argument_type: "path".to_string(),
                    default: None,
                },
            ],
            examples: vec![
                r#"command: /bin/false"#.to_string(),
                r#"command:
    cmd: /usr/bin/make
    chdir: /tmp/project"#.to_string(),
            ],
            return_values: vec![
                ReturnValueSpec {
                    name: "stdout".to_string(),
                    description: "Standard output from command".to_string(),
                    returned: "always".to_string(),
                    value_type: "str".to_string(),
                },
                ReturnValueSpec {
                    name: "stderr".to_string(),
                    description: "Standard error from command".to_string(),
                    returned: "always".to_string(),
                    value_type: "str".to_string(),
                },
                ReturnValueSpec {
                    name: "rc".to_string(),
                    description: "Return code of command".to_string(),
                    returned: "always".to_string(),
                    value_type: "int".to_string(),
                },
            ],
        }
    }
}

impl CommandModule {
    fn extract_command(&self, args: &ModuleArgs) -> Result<Vec<String>, ModuleExecutionError> {
        if let Some(raw_params) = args.args.get("_raw_params") {
            if let Some(cmd_str) = raw_params.as_str() {
                // Split command respecting quotes
                return Ok(shell_words::split(cmd_str)?);
            }
        }

        if let Some(cmd) = args.args.get("cmd") {
            if let Some(cmd_str) = cmd.as_str() {
                return Ok(shell_words::split(cmd_str)?);
            }
            if let Some(cmd_array) = cmd.as_array() {
                return Ok(cmd_array
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect());
            }
        }

        Err(ModuleExecutionError::InvalidArgs {
            message: "No command specified".to_string(),
        })
    }

    fn build_command(
        &self,
        command: &[String],
        context: &ExecutionContext,
    ) -> Result<Command, ModuleExecutionError> {
        if command.is_empty() {
            return Err(ModuleExecutionError::InvalidArgs {
                message: "Empty command".to_string(),
            });
        }

        let mut cmd = Command::new(&command[0]);
        if command.len() > 1 {
            cmd.args(&command[1..]);
        }

        // Set environment variables
        for (key, value) in &context.environment {
            cmd.env(key, value);
        }

        Ok(cmd)
    }
}
