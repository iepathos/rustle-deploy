use anyhow::Result;
use jsonschema::{Draft, JSONSchema};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RustlePlanParseError {
    #[error("Invalid JSON format: {reason}")]
    InvalidJson { reason: String },

    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Invalid field format: {field} expected {expected}, got {actual}")]
    InvalidFieldFormat {
        field: String,
        expected: String,
        actual: String,
    },

    #[error("Schema validation failed: {errors:?}")]
    SchemaValidation { errors: Vec<String> },

    #[error("Unsupported rustle-plan version: {version}")]
    UnsupportedVersion { version: String },

    #[error("Malformed task definition: {task_id} - {reason}")]
    MalformedTask { task_id: String, reason: String },

    #[error("Invalid execution strategy: {strategy}")]
    InvalidStrategy { strategy: String },
}

#[derive(Debug, Error)]
pub enum ConversionError {
    #[error("Failed to convert play structure: {play_id} - {reason}")]
    PlayConversion { play_id: String, reason: String },

    #[error("Task conversion failed: {task_id} - {reason}")]
    TaskConversion { task_id: String, reason: String },

    #[error("Missing inventory information for host: {host}")]
    MissingInventory { host: String },

    #[error("Binary deployment extraction failed: {reason}")]
    BinaryDeploymentExtraction { reason: String },
}

#[derive(Debug, Error)]
pub enum AnalysisError {
    #[error("Binary compatibility analysis failed for task {task_id}: {reason}")]
    CompatibilityAnalysis { task_id: String, reason: String },

    #[error("Architecture detection failed for hosts: {hosts:?}")]
    ArchitectureDetection { hosts: Vec<String> },

    #[error("Module dependency resolution failed: {module} - {reason}")]
    ModuleDependency { module: String, reason: String },

    #[error("Network efficiency calculation failed: {reason}")]
    NetworkEfficiency { reason: String },

    #[error("Time estimation failed: {0}")]
    Estimation(#[from] EstimationError),

    #[error("Calculation failed: {0}")]
    Calculation(#[from] CalculationError),

    #[error("Assessment failed: {0}")]
    Assessment(#[from] AssessmentError),
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Schema validation failed: {details}")]
    Schema { details: String },

    #[error("Semantic validation failed: {field} - {reason}")]
    Semantic { field: String, reason: String },

    #[error("Reference validation failed: {reference} - {reason}")]
    Reference { reference: String, reason: String },
}

#[derive(Debug, Error)]
pub enum EstimationError {
    #[error("Duration estimation failed: {reason}")]
    Duration { reason: String },

    #[error("Resource estimation failed: {resource} - {reason}")]
    Resource { resource: String, reason: String },
}

#[derive(Debug, Error)]
pub enum CalculationError {
    #[error("Network efficiency calculation failed: {reason}")]
    NetworkEfficiency { reason: String },

    #[error("Performance calculation failed: {metric} - {reason}")]
    Performance { metric: String, reason: String },
}

#[derive(Debug, Error)]
pub enum AssessmentError {
    #[error("Compatibility assessment failed: {reason}")]
    Compatibility { reason: String },

    #[error("Risk assessment failed: {reason}")]
    Risk { reason: String },
}

#[derive(Debug, Error)]
pub enum ExtractionError {
    #[error("Binary deployment extraction failed: {reason}")]
    BinaryDeployment { reason: String },

    #[error("Task dependency extraction failed: {reason}")]
    TaskDependency { reason: String },
}

pub struct SchemaValidator {
    schema: JSONSchema,
}

impl SchemaValidator {
    pub fn new() -> Result<Self> {
        let schema_json = Self::get_rustle_plan_schema();
        let schema = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&schema_json)
            .map_err(|e| anyhow::anyhow!("Failed to compile schema: {}", e))?;

        Ok(Self { schema })
    }

    pub fn validate(&self, value: &Value) -> Result<(), ValidationError> {
        let result = self.schema.validate(value);

        if let Err(errors) = result {
            let error_messages: Vec<String> = errors
                .map(|error| format!("{}: {}", error.instance_path, error))
                .collect();

            return Err(ValidationError::Schema {
                details: error_messages.join("; "),
            });
        }

        Ok(())
    }

    fn get_rustle_plan_schema() -> Value {
        serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "required": ["metadata", "plays", "total_tasks", "hosts"],
            "properties": {
                "metadata": {
                    "type": "object",
                    "required": ["created_at", "rustle_version", "playbook_hash", "inventory_hash", "planning_options"],
                    "properties": {
                        "created_at": { "type": "string", "format": "date-time" },
                        "rustle_version": { "type": "string" },
                        "playbook_hash": { "type": "string" },
                        "inventory_hash": { "type": "string" },
                        "planning_options": {
                            "type": "object",
                            "required": ["forks", "strategy", "binary_threshold"],
                            "properties": {
                                "limit": { "type": ["string", "null"] },
                                "tags": { "type": "array", "items": { "type": "string" } },
                                "skip_tags": { "type": "array", "items": { "type": "string" } },
                                "check_mode": { "type": "boolean" },
                                "diff_mode": { "type": "boolean" },
                                "forks": { "type": "integer", "minimum": 1 },
                                "serial": { "type": ["integer", "null"] },
                                "strategy": {
                                    "type": "string",
                                    "enum": ["Linear", "Free", "BinaryHybrid", "BinaryOnly", "SshOnly"]
                                },
                                "binary_threshold": { "type": "integer", "minimum": 1 },
                                "force_binary": { "type": "boolean" },
                                "force_ssh": { "type": "boolean" }
                            }
                        }
                    }
                },
                "plays": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["play_id", "name", "strategy", "hosts", "batches", "handlers"],
                        "properties": {
                            "play_id": { "type": "string" },
                            "name": { "type": "string" },
                            "strategy": {
                                "type": "string",
                                "enum": ["Linear", "Free", "BinaryHybrid", "BinaryOnly", "SshOnly"]
                            },
                            "hosts": { "type": "array", "items": { "type": "string" } },
                            "batches": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "required": ["batch_id", "hosts", "tasks"],
                                    "properties": {
                                        "batch_id": { "type": "string" },
                                        "hosts": { "type": "array", "items": { "type": "string" } },
                                        "tasks": {
                                            "type": "array",
                                            "items": {
                                                "type": "object",
                                                "required": ["task_id", "name", "module", "args", "hosts", "dependencies", "conditions", "tags", "notify", "execution_order", "can_run_parallel", "estimated_duration", "risk_level"],
                                                "properties": {
                                                    "task_id": { "type": "string" },
                                                    "name": { "type": "string" },
                                                    "module": { "type": "string" },
                                                    "args": { "type": "object" },
                                                    "hosts": { "type": "array", "items": { "type": "string" } },
                                                    "dependencies": { "type": "array", "items": { "type": "string" } },
                                                    "conditions": { "type": "array" },
                                                    "tags": { "type": "array", "items": { "type": "string" } },
                                                    "notify": { "type": "array", "items": { "type": "string" } },
                                                    "execution_order": { "type": "integer", "minimum": 0 },
                                                    "can_run_parallel": { "type": "boolean" },
                                                    "estimated_duration": {
                                                        "type": "object",
                                                        "required": ["secs", "nanos"],
                                                        "properties": {
                                                            "secs": { "type": "integer", "minimum": 0 },
                                                            "nanos": { "type": "integer", "minimum": 0, "maximum": 999999999 }
                                                        }
                                                    },
                                                    "risk_level": {
                                                        "type": "string",
                                                        "enum": ["Low", "Medium", "High", "Critical"]
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            "handlers": { "type": "array" }
                        }
                    }
                },
                "binary_deployments": { "type": "array" },
                "total_tasks": { "type": "integer", "minimum": 0 },
                "estimated_duration": {
                    "type": ["object", "null"],
                    "properties": {
                        "secs": { "type": "integer", "minimum": 0 },
                        "nanos": { "type": "integer", "minimum": 0, "maximum": 999999999 }
                    }
                },
                "estimated_compilation_time": {
                    "type": ["object", "null"],
                    "properties": {
                        "secs": { "type": "integer", "minimum": 0 },
                        "nanos": { "type": "integer", "minimum": 0, "maximum": 999999999 }
                    }
                },
                "parallelism_score": { "type": "number", "minimum": 0.0, "maximum": 1.0 },
                "network_efficiency_score": { "type": "number", "minimum": 0.0, "maximum": 1.0 },
                "hosts": { "type": "array", "items": { "type": "string" } }
            }
        })
    }
}

impl Default for SchemaValidator {
    fn default() -> Self {
        Self::new().expect("Failed to create default schema validator")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_validator_creation() {
        let validator = SchemaValidator::new();
        assert!(validator.is_ok());
    }

    #[test]
    fn test_valid_rustle_plan_schema() {
        let validator = SchemaValidator::new().unwrap();
        let valid_plan = serde_json::json!({
            "metadata": {
                "created_at": "2025-07-11T05:18:16.945474Z",
                "rustle_version": "0.1.0",
                "playbook_hash": "test",
                "inventory_hash": "test",
                "planning_options": {
                    "forks": 5,
                    "strategy": "Linear",
                    "binary_threshold": 1
                }
            },
            "plays": [],
            "binary_deployments": [],
            "total_tasks": 0,
            "hosts": []
        });

        let result = validator.validate(&valid_plan);
        assert!(result.is_ok());
    }
}
