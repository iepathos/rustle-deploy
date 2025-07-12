//! Template module for processing templates with variable substitution

use async_trait::async_trait;
use handlebars::Handlebars;
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
    atomic::AtomicWriter, backup::create_backup, ownership::set_ownership,
    permissions::set_permissions, FileError,
};

/// Template module arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateArgs {
    pub src: String,                          // Required: template file path
    pub dest: String,                         // Required: destination file path
    pub backup: Option<bool>,                 // Backup destination before writing
    pub mode: Option<String>,                 // File permissions
    pub owner: Option<String>,                // File owner
    pub group: Option<String>,                // File group
    pub validate: Option<String>,             // Validation command
    pub variables: Option<serde_json::Value>, // Template variables
}

impl TemplateArgs {
    pub fn from_module_args(args: &ModuleArgs) -> Result<Self, ValidationError> {
        let mut template_args = Self {
            src: String::new(),
            dest: String::new(),
            backup: None,
            mode: None,
            owner: None,
            group: None,
            validate: None,
            variables: None,
        };

        // Required src
        if let Some(src) = args.args.get("src") {
            template_args.src = src
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
            template_args.dest = dest
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
            template_args.backup = backup.as_bool();
        }

        if let Some(mode) = args.args.get("mode") {
            template_args.mode = mode.as_str().map(|s| s.to_string());
        }

        if let Some(owner) = args.args.get("owner") {
            template_args.owner = owner.as_str().map(|s| s.to_string());
        }

        if let Some(group) = args.args.get("group") {
            template_args.group = group.as_str().map(|s| s.to_string());
        }

        if let Some(validate) = args.args.get("validate") {
            template_args.validate = validate.as_str().map(|s| s.to_string());
        }

        if let Some(variables) = args.args.get("variables") {
            template_args.variables = Some(variables.clone());
        }

        Ok(template_args)
    }
}

/// Template processor with Handlebars
pub struct TemplateProcessor {
    handlebars: Handlebars<'static>,
}

impl TemplateProcessor {
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);

        Self { handlebars }
    }

    pub fn render_template(
        &self,
        template_content: &str,
        variables: &serde_json::Value,
    ) -> Result<String, handlebars::RenderError> {
        self.handlebars.render_template(template_content, variables)
    }
}

/// Template module implementation
pub struct TemplateModule;

#[async_trait]
impl ExecutionModule for TemplateModule {
    fn name(&self) -> &'static str {
        "template"
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
        let template_args = TemplateArgs::from_module_args(args)
            .map_err(|e| ModuleExecutionError::Validation(e))?;

        self.execute_template_operation(&template_args, context)
            .await
    }

    fn validate_args(&self, args: &ModuleArgs) -> Result<(), ValidationError> {
        TemplateArgs::from_module_args(args)?;
        Ok(())
    }

    async fn check_mode(
        &self,
        args: &ModuleArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        let template_args = TemplateArgs::from_module_args(args)
            .map_err(|e| ModuleExecutionError::Validation(e))?;

        self.analyze_template_operation(&template_args, context)
            .await
    }

    fn documentation(&self) -> ModuleDocumentation {
        ModuleDocumentation {
            description: "Process templates with variable substitution using Handlebars"
                .to_string(),
            arguments: vec![
                ArgumentSpec {
                    name: "src".to_string(),
                    description: "Path to the template file".to_string(),
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
                    name: "mode".to_string(),
                    description: "Set permissions on destination file".to_string(),
                    required: false,
                    argument_type: "str".to_string(),
                    default: None,
                },
                ArgumentSpec {
                    name: "variables".to_string(),
                    description: "Variables to use in template processing".to_string(),
                    required: false,
                    argument_type: "dict".to_string(),
                    default: None,
                },
                ArgumentSpec {
                    name: "validate".to_string(),
                    description:
                        "Command to validate generated file (%s will be replaced with file path)"
                            .to_string(),
                    required: false,
                    argument_type: "str".to_string(),
                    default: None,
                },
            ],
            examples: vec![r#"template:
  src: nginx.conf.j2
  dest: /etc/nginx/nginx.conf
  backup: yes
  variables:
    server_name: example.com
    worker_processes: 4"#
                .to_string()],
            return_values: vec![
                ReturnValueSpec {
                    name: "changed".to_string(),
                    description: "Whether the file was changed".to_string(),
                    returned: "always".to_string(),
                    value_type: "bool".to_string(),
                },
                ReturnValueSpec {
                    name: "src".to_string(),
                    description: "Template source file path".to_string(),
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

impl TemplateModule {
    async fn execute_template_operation(
        &self,
        args: &TemplateArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        let src_path = Path::new(&args.src);
        let dest_path = Path::new(&args.dest);
        let mut changed = false;
        let mut results = HashMap::new();

        // Check if source template exists
        if !src_path.exists() {
            return Err(ModuleExecutionError::ExecutionFailed {
                message: format!("Template file does not exist: {}", args.src),
            });
        }

        // Read template content
        let template_content = fs::read_to_string(src_path).await.map_err(|e| {
            ModuleExecutionError::ExecutionFailed {
                message: format!("Failed to read template file: {}", e),
            }
        })?;

        // Prepare template variables
        let mut template_vars = serde_json::Map::new();

        // Add context variables
        for (key, value) in &context.variables {
            template_vars.insert(key.clone(), value.clone());
        }

        // Add context facts
        for (key, value) in &context.facts {
            template_vars.insert(format!("ansible_{}", key), value.clone());
        }

        // Add host information
        template_vars.insert(
            "inventory_hostname".to_string(),
            serde_json::Value::String(context.host_info.hostname.clone()),
        );
        template_vars.insert(
            "ansible_os_family".to_string(),
            serde_json::Value::String(context.host_info.os_family.clone()),
        );
        template_vars.insert(
            "ansible_architecture".to_string(),
            serde_json::Value::String(context.host_info.architecture.clone()),
        );

        // Add user-provided variables (these override context variables)
        if let Some(user_vars) = &args.variables {
            if let serde_json::Value::Object(user_map) = user_vars {
                for (key, value) in user_map {
                    template_vars.insert(key.clone(), value.clone());
                }
            }
        }

        let variables = serde_json::Value::Object(template_vars);

        // Process template
        let processor = TemplateProcessor::new();
        let rendered_content = processor
            .render_template(&template_content, &variables)
            .map_err(|e| ModuleExecutionError::ExecutionFailed {
                message: format!("Template rendering failed: {}", e),
            })?;

        // Check if destination content would be different
        let content_changed = if dest_path.exists() {
            let existing_content = fs::read_to_string(dest_path).await.map_err(|e| {
                ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to read existing destination file: {}", e),
                }
            })?;
            existing_content != rendered_content
        } else {
            true // File doesn't exist, so it will change
        };

        if content_changed {
            // Create backup if requested and destination exists
            if args.backup.unwrap_or(false) && dest_path.exists() {
                if let Ok(Some(backup_path)) = create_backup(dest_path, None).await {
                    results.insert(
                        "backup_file".to_string(),
                        serde_json::Value::String(backup_path.display().to_string()),
                    );
                }
            }

            // Create destination directory if it doesn't exist
            if let Some(parent_dir) = dest_path.parent() {
                if !parent_dir.exists() {
                    fs::create_dir_all(parent_dir).await.map_err(|e| {
                        ModuleExecutionError::ExecutionFailed {
                            message: format!("Failed to create destination directory: {}", e),
                        }
                    })?;
                }
            }

            // Write rendered content atomically
            let mut writer = AtomicWriter::new(dest_path).await.map_err(|e| {
                ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to create atomic writer: {}", e),
                }
            })?;

            writer
                .write_all(rendered_content.as_bytes())
                .await
                .map_err(|e| ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to write template output: {}", e),
                })?;

            writer
                .commit()
                .await
                .map_err(|e| ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to commit template file: {}", e),
                })?;

            changed = true;
        }

        // Set permissions if specified
        if let Some(mode) = &args.mode {
            set_permissions(dest_path, mode).await.map_err(|e| {
                ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to set file permissions: {}", e),
                }
            })?;
        }

        // Set ownership if specified
        if args.owner.is_some() || args.group.is_some() {
            set_ownership(dest_path, args.owner.as_deref(), args.group.as_deref())
                .await
                .map_err(|e| ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to set file ownership: {}", e),
                })?;
        }

        // Run validation command if specified
        if let Some(validate_cmd) = &args.validate {
            let cmd = validate_cmd.replace("%s", &dest_path.to_string_lossy());
            let output = tokio::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .output()
                .await
                .map_err(|e| ModuleExecutionError::ExecutionFailed {
                    message: format!("Failed to run validation command: {}", e),
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

        results.insert(
            "src".to_string(),
            serde_json::Value::String(args.src.clone()),
        );
        results.insert(
            "dest".to_string(),
            serde_json::Value::String(args.dest.clone()),
        );

        Ok(ModuleResult {
            changed,
            failed: false,
            msg: Some("Template processed successfully".to_string()),
            stdout: None,
            stderr: None,
            rc: Some(0),
            results,
            diff: None,
            warnings: vec![],
            ansible_facts: HashMap::new(),
        })
    }

    async fn analyze_template_operation(
        &self,
        args: &TemplateArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        let src_path = Path::new(&args.src);
        let dest_path = Path::new(&args.dest);
        let mut results = HashMap::new();

        let src_exists = src_path.exists();
        let dest_exists = dest_path.exists();

        let would_change = if !src_exists {
            false // Can't process non-existent template
        } else if !dest_exists {
            true // Would create new file
        } else {
            // Would need to render template and compare content
            // For check mode, we'll assume it would change
            true
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::interface::HostInfo;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tokio::io::AsyncWriteExt;

    fn create_test_context() -> ExecutionContext {
        let mut variables = HashMap::new();
        variables.insert(
            "app_name".to_string(),
            serde_json::Value::String("test_app".to_string()),
        );
        variables.insert(
            "port".to_string(),
            serde_json::Value::Number(serde_json::Number::from(8080)),
        );

        ExecutionContext {
            facts: HashMap::new(),
            variables,
            host_info: HostInfo::detect(),
            working_directory: PathBuf::from("/tmp"),
            environment: HashMap::new(),
            check_mode: false,
            diff_mode: false,
            verbosity: 0,
        }
    }

    #[tokio::test]
    async fn test_template_processing() {
        let temp_dir = TempDir::new().unwrap();
        let template_path = temp_dir.path().join("test.conf.j2");
        let dest_path = temp_dir.path().join("test.conf");

        // Create template file
        let template_content = r#"
app_name = {{app_name}}
port = {{port}}
hostname = {{inventory_hostname}}
"#;
        let mut template_file = tokio::fs::File::create(&template_path).await.unwrap();
        template_file
            .write_all(template_content.as_bytes())
            .await
            .unwrap();
        template_file.flush().await.unwrap();

        let args = ModuleArgs {
            args: {
                let mut map = HashMap::new();
                map.insert(
                    "src".to_string(),
                    serde_json::Value::String(template_path.to_string_lossy().to_string()),
                );
                map.insert(
                    "dest".to_string(),
                    serde_json::Value::String(dest_path.to_string_lossy().to_string()),
                );
                map
            },
            special: Default::default(),
        };

        let module = TemplateModule;
        let context = create_test_context();
        let result = module.execute(&args, &context).await.unwrap();

        assert!(result.changed);
        assert!(dest_path.exists());

        let rendered_content = tokio::fs::read_to_string(&dest_path).await.unwrap();
        assert!(rendered_content.contains("app_name = test_app"));
        assert!(rendered_content.contains("port = 8080"));
        assert!(rendered_content.contains(&format!("hostname = {}", context.host_info.hostname)));
    }

    #[tokio::test]
    async fn test_template_with_custom_variables() {
        let temp_dir = TempDir::new().unwrap();
        let template_path = temp_dir.path().join("custom.conf.j2");
        let dest_path = temp_dir.path().join("custom.conf");

        // Create template file
        let template_content = r#"custom_var = {{custom_var}}"#;
        tokio::fs::write(&template_path, template_content)
            .await
            .unwrap();

        let args = ModuleArgs {
            args: {
                let mut map = HashMap::new();
                map.insert(
                    "src".to_string(),
                    serde_json::Value::String(template_path.to_string_lossy().to_string()),
                );
                map.insert(
                    "dest".to_string(),
                    serde_json::Value::String(dest_path.to_string_lossy().to_string()),
                );
                map.insert(
                    "variables".to_string(),
                    serde_json::json!({
                        "custom_var": "custom_value"
                    }),
                );
                map
            },
            special: Default::default(),
        };

        let module = TemplateModule;
        let context = create_test_context();
        let result = module.execute(&args, &context).await.unwrap();

        assert!(result.changed);

        let rendered_content = tokio::fs::read_to_string(&dest_path).await.unwrap();
        assert_eq!(rendered_content.trim(), "custom_var = custom_value");
    }

    #[tokio::test]
    async fn test_template_unchanged_content() {
        let temp_dir = TempDir::new().unwrap();
        let template_path = temp_dir.path().join("static.conf.j2");
        let dest_path = temp_dir.path().join("static.conf");

        // Create template file with static content
        let template_content = "static content";
        tokio::fs::write(&template_path, template_content)
            .await
            .unwrap();

        // Create destination file with same content
        tokio::fs::write(&dest_path, template_content)
            .await
            .unwrap();

        let args = ModuleArgs {
            args: {
                let mut map = HashMap::new();
                map.insert(
                    "src".to_string(),
                    serde_json::Value::String(template_path.to_string_lossy().to_string()),
                );
                map.insert(
                    "dest".to_string(),
                    serde_json::Value::String(dest_path.to_string_lossy().to_string()),
                );
                map
            },
            special: Default::default(),
        };

        let module = TemplateModule;
        let context = create_test_context();
        let result = module.execute(&args, &context).await.unwrap();

        assert!(!result.changed); // Content is the same, no change needed
    }
}
