use crate::execution::rustle_plan::{BinaryDeploymentPlan, RustlePlanOutput};
use thiserror::Error;

/// Configuration for format migration behavior
#[derive(Debug, Clone)]
pub struct MigrationConfig {
    pub strict_mode: bool,            // Fail on any migration issues
    pub preserve_legacy_fields: bool, // Keep old fields for debugging
    pub validate_embedded_data: bool, // Deep validation of embedded content
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            strict_mode: false,
            preserve_legacy_fields: true,
            validate_embedded_data: true,
        }
    }
}

/// Errors that can occur during format migration
#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("Unsupported format version: {version}")]
    UnsupportedVersion { version: String },

    #[error("Missing required field in new format: {field}")]
    MissingRequiredField { field: String },

    #[error("Invalid embedded execution plan: {reason}")]
    InvalidEmbeddedPlan { reason: String },

    #[error("Target architecture parsing failed: {target_triple}")]
    TargetArchitectureParsingFailed { target_triple: String },

    #[error("JSON parsing error in embedded data: {error}")]
    JsonParsingError { error: String },
}

/// Warnings that can occur during migration but don't prevent completion
#[derive(Debug)]
pub enum MigrationWarning {
    TaskMigrationFailed(String),
    CompilationRequirementsMigrationFailed(String),
    EmbeddedDataValidationFailed(String),
}

/// Handles migration between different rustle-plan format versions
pub struct FormatMigrator {
    config: MigrationConfig,
}

impl FormatMigrator {
    pub fn new() -> Self {
        Self {
            config: MigrationConfig::default(),
        }
    }

    pub fn with_config(config: MigrationConfig) -> Self {
        Self { config }
    }

    /// Migrate a complete RustlePlanOutput to the latest format
    pub fn migrate_rustle_plan_output(
        &self,
        plan: &mut RustlePlanOutput,
    ) -> Result<Vec<MigrationWarning>, MigrationError> {
        let mut warnings = Vec::new();

        for deployment in &mut plan.binary_deployments {
            match self.migrate_binary_deployment_plan(deployment) {
                Ok(mut deployment_warnings) => {
                    warnings.append(&mut deployment_warnings);
                }
                Err(e) => {
                    if self.config.strict_mode {
                        return Err(e);
                    } else {
                        warnings.push(MigrationWarning::TaskMigrationFailed(e.to_string()));
                    }
                }
            }
        }

        Ok(warnings)
    }

    /// Migrate a single BinaryDeploymentPlan to the latest format
    pub fn migrate_binary_deployment_plan(
        &self,
        plan: &mut BinaryDeploymentPlan,
    ) -> Result<Vec<MigrationWarning>, MigrationError> {
        let mut warnings = Vec::new();

        // Migrate task_ids to tasks
        if let Err(e) = self.migrate_tasks(plan) {
            if self.config.strict_mode {
                return Err(e);
            } else {
                warnings.push(MigrationWarning::TaskMigrationFailed(e.to_string()));
            }
        }

        // Migrate compilation requirements
        if let Err(e) = self.migrate_compilation_requirements(plan) {
            if self.config.strict_mode {
                return Err(e);
            } else {
                warnings.push(MigrationWarning::CompilationRequirementsMigrationFailed(
                    e.to_string(),
                ));
            }
        }

        // Set binary_name if not present
        if plan.binary_name.is_empty() {
            plan.binary_name = format!("rustle-runner-{}", plan.deployment_id);
        }

        // Validate embedded data if requested
        if self.config.validate_embedded_data {
            if let Err(e) = self.validate_embedded_data(plan) {
                if self.config.strict_mode {
                    return Err(e);
                } else {
                    warnings.push(MigrationWarning::EmbeddedDataValidationFailed(
                        e.to_string(),
                    ));
                }
            }
        }

        Ok(warnings)
    }

    /// Migrate with fallback strategy - never fails, always tries to recover
    pub fn migrate_with_fallback(
        &self,
        plan: &mut BinaryDeploymentPlan,
    ) -> Result<Vec<MigrationWarning>, MigrationError> {
        let mut warnings = Vec::new();

        // Try to migrate each component independently
        if let Err(e) = self.migrate_tasks(plan) {
            warnings.push(MigrationWarning::TaskMigrationFailed(e.to_string()));
            // Use fallback: if we have task_ids, copy them to tasks
            if let Some(ref task_ids) = plan.task_ids {
                plan.tasks = task_ids.clone();
            }
        }

        if let Err(e) = self.migrate_compilation_requirements(plan) {
            warnings.push(MigrationWarning::CompilationRequirementsMigrationFailed(
                e.to_string(),
            ));
            // Use fallback: set sensible defaults
            if plan.compilation_requirements.target_arch.is_empty() {
                plan.compilation_requirements.target_arch = "x86_64".to_string();
            }
            if plan.compilation_requirements.target_os.is_empty() {
                plan.compilation_requirements.target_os = "linux".to_string();
            }
        }

        // Ensure binary_name is set
        if plan.binary_name.is_empty() {
            plan.binary_name = format!("rustle-runner-{}", plan.deployment_id);
        }

        Ok(warnings)
    }

    fn migrate_tasks(&self, plan: &mut BinaryDeploymentPlan) -> Result<(), MigrationError> {
        if let Some(ref task_ids) = plan.task_ids {
            if plan.tasks.is_empty() {
                plan.tasks = task_ids.clone();
            }
        }

        if plan.tasks.is_empty() && plan.task_ids.is_none() {
            return Err(MigrationError::MissingRequiredField {
                field: "tasks or task_ids".to_string(),
            });
        }

        Ok(())
    }

    fn migrate_compilation_requirements(
        &self,
        plan: &mut BinaryDeploymentPlan,
    ) -> Result<(), MigrationError> {
        let reqs = &mut plan.compilation_requirements;

        // If new format fields are empty but legacy fields exist, migrate
        if reqs.target_arch.is_empty() || reqs.target_os.is_empty() {
            if let Some(ref target_triple) = reqs.target_triple {
                let (arch, os) = self.parse_target_triple(target_triple)?;
                if reqs.target_arch.is_empty() {
                    reqs.target_arch = arch;
                }
                if reqs.target_os.is_empty() {
                    reqs.target_os = os;
                }
            } else {
                // No legacy data available, use defaults
                if reqs.target_arch.is_empty() {
                    reqs.target_arch = "x86_64".to_string();
                }
                if reqs.target_os.is_empty() {
                    reqs.target_os = "linux".to_string();
                }
            }
        }

        // Set default rust_version if not present
        if reqs.rust_version.is_empty() {
            reqs.rust_version = "1.70.0".to_string();
        }

        Ok(())
    }

    fn parse_target_triple(&self, triple: &str) -> Result<(String, String), MigrationError> {
        let parts: Vec<&str> = triple.split('-').collect();
        if parts.len() >= 3 {
            Ok((parts[0].to_string(), parts[2].to_string()))
        } else {
            Err(MigrationError::TargetArchitectureParsingFailed {
                target_triple: triple.to_string(),
            })
        }
    }

    fn validate_embedded_data(&self, plan: &BinaryDeploymentPlan) -> Result<(), MigrationError> {
        // Validate execution plan JSON if present
        if !plan.embedded_data.execution_plan.is_empty() {
            if let Err(e) =
                serde_json::from_str::<serde_json::Value>(&plan.embedded_data.execution_plan)
            {
                return Err(MigrationError::JsonParsingError {
                    error: e.to_string(),
                });
            }
        }

        // Validate static file references
        for static_file in &plan.embedded_data.static_files {
            if static_file.src_path.is_empty() || static_file.dest_path.is_empty() {
                return Err(MigrationError::InvalidEmbeddedPlan {
                    reason: "Static file missing src_path or dest_path".to_string(),
                });
            }
        }

        Ok(())
    }
}

impl Default for FormatMigrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::rustle_plan::{BinaryDeploymentPlan, CompilationRequirements};
    use std::time::Duration;

    fn create_legacy_deployment_plan() -> BinaryDeploymentPlan {
        BinaryDeploymentPlan {
            deployment_id: "test_deployment".to_string(),
            target_hosts: vec!["localhost".to_string()],
            binary_name: String::new(), // Empty - should be migrated
            tasks: vec![],              // Empty - should be migrated from task_ids
            modules: vec![],
            embedded_data: Default::default(),
            execution_mode: Default::default(),
            estimated_size: 0,
            compilation_requirements: CompilationRequirements {
                target_arch: String::new(),  // Empty - should be migrated
                target_os: String::new(),    // Empty - should be migrated
                rust_version: String::new(), // Empty - should be migrated
                cross_compilation: false,
                static_linking: true,
                modules: Some(vec!["file".to_string(), "copy".to_string()]),
                static_files: Some(vec![]),
                target_triple: Some("x86_64-unknown-linux-gnu".to_string()),
                optimization_level: Some("release".to_string()),
                features: Some(vec![]),
            },
            task_ids: Some(vec!["task1".to_string(), "task2".to_string()]),
            target_architecture: Some("x86_64-linux".to_string()),
            estimated_savings: Some(Duration::from_secs(10)),
            controller_endpoint: None,
            execution_timeout: None,
            report_interval: None,
            cleanup_on_completion: Some(true),
            log_level: Some("info".to_string()),
            max_retries: Some(3),
            static_files: vec![],
            secrets: vec![],
            verbose: Some(false),
        }
    }

    #[test]
    fn test_migration_from_legacy_format() {
        let mut plan = create_legacy_deployment_plan();
        let migrator = FormatMigrator::new();

        let warnings = migrator.migrate_binary_deployment_plan(&mut plan).unwrap();

        // Should have no warnings for valid migration
        assert!(warnings.is_empty());

        // Check that migration worked
        assert_eq!(plan.tasks, vec!["task1", "task2"]);
        assert_eq!(plan.binary_name, "rustle-runner-test_deployment");
        assert_eq!(plan.compilation_requirements.target_arch, "x86_64");
        assert_eq!(plan.compilation_requirements.target_os, "linux");
        assert_eq!(plan.compilation_requirements.rust_version, "1.70.0");
    }

    #[test]
    fn test_migration_with_fallback() {
        let mut plan = BinaryDeploymentPlan {
            deployment_id: "test".to_string(),
            target_hosts: vec!["localhost".to_string()],
            binary_name: String::new(),
            tasks: vec![],
            modules: vec![],
            embedded_data: Default::default(),
            execution_mode: Default::default(),
            estimated_size: 0,
            compilation_requirements: Default::default(),
            task_ids: None, // No legacy data
            target_architecture: None,
            estimated_savings: None,
            controller_endpoint: None,
            execution_timeout: None,
            report_interval: None,
            cleanup_on_completion: None,
            log_level: None,
            max_retries: None,
            static_files: vec![],
            secrets: vec![],
            verbose: None,
        };

        let migrator = FormatMigrator::new();
        let warnings = migrator.migrate_with_fallback(&mut plan).unwrap();

        // Should have warnings for missing data
        assert!(!warnings.is_empty());

        // But migration should still succeed with defaults
        assert_eq!(plan.binary_name, "rustle-runner-test");
        assert_eq!(plan.compilation_requirements.target_arch, "x86_64");
        assert_eq!(plan.compilation_requirements.target_os, "linux");
    }

    #[test]
    fn test_target_triple_parsing() {
        let migrator = FormatMigrator::new();

        let (arch, os) = migrator
            .parse_target_triple("x86_64-unknown-linux-gnu")
            .unwrap();
        assert_eq!(arch, "x86_64");
        assert_eq!(os, "linux");

        let (arch, os) = migrator
            .parse_target_triple("aarch64-apple-darwin")
            .unwrap();
        assert_eq!(arch, "aarch64");
        assert_eq!(os, "darwin");

        // Test invalid format
        assert!(migrator.parse_target_triple("invalid").is_err());
    }

    #[test]
    fn test_embedded_data_validation() {
        let migrator = FormatMigrator::with_config(MigrationConfig {
            validate_embedded_data: true,
            ..Default::default()
        });

        let mut plan = BinaryDeploymentPlan::default();
        plan.embedded_data.execution_plan = r#"{"tasks": []}"#.to_string();

        // Should validate successfully
        assert!(migrator.validate_embedded_data(&plan).is_ok());

        // Test invalid JSON
        plan.embedded_data.execution_plan = "invalid json".to_string();
        assert!(migrator.validate_embedded_data(&plan).is_err());
    }
}
