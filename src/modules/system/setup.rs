//! Setup module for comprehensive system fact gathering

use crate::modules::error::{ModuleExecutionError, ValidationError};
use crate::modules::interface::{
    ArgumentSpec, ExecutionContext, ExecutionModule, ModuleArgs, ModuleDocumentation, ModuleResult,
    Platform, ReturnValueSpec,
};
use crate::modules::system::facts::{
    collector::{FactCollector, SystemFactCollector},
    FactCategory, SystemFacts,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

pub struct SetupModule;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupArgs {
    #[serde(default)]
    pub gather_subset: Option<Vec<FactCategory>>,
    #[serde(default)]
    pub gather_timeout: Option<u64>,
    #[serde(default)]
    pub filter: Option<Vec<String>>,
    #[serde(default)]
    pub fact_path: Option<String>,
}

impl Default for SetupArgs {
    fn default() -> Self {
        Self {
            gather_subset: Some(vec![FactCategory::Default]),
            gather_timeout: Some(30),
            filter: None,
            fact_path: None,
        }
    }
}

impl SetupModule {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ExecutionModule for SetupModule {
    fn name(&self) -> &'static str {
        "setup"
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
        _context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        let setup_args = self.parse_args(args)?;

        // Configure collector based on arguments
        let mut collector = SystemFactCollector::new();

        if let Some(timeout) = setup_args.gather_timeout {
            collector = collector.with_timeout(Duration::from_secs(timeout));
        }

        if let Some(fact_path) = setup_args.fact_path {
            collector = collector.with_custom_fact_paths(vec![PathBuf::from(fact_path)]);
        }

        // Determine what facts to gather
        let gather_subset = setup_args
            .gather_subset
            .unwrap_or_else(|| vec![FactCategory::Default]);

        // Collect facts
        let facts = collector.collect_facts(&gather_subset).await.map_err(|e| {
            ModuleExecutionError::ExecutionFailed {
                message: e.to_string(),
            }
        })?;

        // Apply filters if specified
        let filtered_facts = if let Some(filters) = setup_args.filter {
            self.apply_filters(facts, &filters)
        } else {
            facts
        };

        // Convert facts to JSON for ModuleResult
        let ansible_facts = self.facts_to_json(filtered_facts)?;

        Ok(ModuleResult {
            changed: false, // Setup module never changes system state
            failed: false,
            msg: Some("Facts gathered successfully".to_string()),
            stdout: None,
            stderr: None,
            rc: Some(0),
            results: HashMap::new(),
            diff: None,
            warnings: Vec::new(),
            ansible_facts,
        })
    }

    fn validate_args(&self, args: &ModuleArgs) -> Result<(), ValidationError> {
        let _setup_args = self
            .parse_args(args)
            .map_err(|e| ValidationError::InvalidArgValue {
                arg: "setup_args".to_string(),
                value: "invalid".to_string(),
                reason: e.to_string(),
            })?;

        Ok(())
    }

    async fn check_mode(
        &self,
        args: &ModuleArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        // Setup module is always safe to run in check mode
        self.execute(args, context).await
    }

    fn documentation(&self) -> ModuleDocumentation {
        ModuleDocumentation {
            description: "Gathers system facts about the target host".to_string(),
            arguments: vec![
                ArgumentSpec {
                    name: "gather_subset".to_string(),
                    description: "List of fact categories to collect".to_string(),
                    required: false,
                    argument_type: "list".to_string(),
                    default: Some("['default']".to_string()),
                },
                ArgumentSpec {
                    name: "gather_timeout".to_string(),
                    description: "Timeout in seconds for fact collection".to_string(),
                    required: false,
                    argument_type: "int".to_string(),
                    default: Some("30".to_string()),
                },
                ArgumentSpec {
                    name: "filter".to_string(),
                    description: "List of fact name patterns to include".to_string(),
                    required: false,
                    argument_type: "list".to_string(),
                    default: None,
                },
                ArgumentSpec {
                    name: "fact_path".to_string(),
                    description: "Path to custom fact scripts directory".to_string(),
                    required: false,
                    argument_type: "path".to_string(),
                    default: None,
                },
            ],
            examples: vec![
                "# Gather all facts\n- setup:".to_string(),
                "# Gather only hardware facts\n- setup:\n    gather_subset:\n      - hardware"
                    .to_string(),
                "# Gather facts with timeout\n- setup:\n    gather_timeout: 60".to_string(),
            ],
            return_values: vec![
                ReturnValueSpec {
                    name: "ansible_facts".to_string(),
                    description: "Dictionary containing all gathered facts".to_string(),
                    returned: "always".to_string(),
                    value_type: "dict".to_string(),
                },
                ReturnValueSpec {
                    name: "ansible_system".to_string(),
                    description: "Operating system name".to_string(),
                    returned: "always".to_string(),
                    value_type: "str".to_string(),
                },
                ReturnValueSpec {
                    name: "ansible_hostname".to_string(),
                    description: "System hostname".to_string(),
                    returned: "always".to_string(),
                    value_type: "str".to_string(),
                },
            ],
        }
    }
}

impl SetupModule {
    fn parse_args(&self, args: &ModuleArgs) -> Result<SetupArgs, ModuleExecutionError> {
        if args.args.is_empty() {
            return Ok(SetupArgs::default());
        }

        let args_value = serde_json::Value::Object(
            args.args
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        );

        serde_json::from_value(args_value).map_err(|e| ModuleExecutionError::InvalidArgs {
            message: e.to_string(),
        })
    }

    fn apply_filters(&self, facts: SystemFacts, _filters: &[String]) -> SystemFacts {
        // This is a simplified filter implementation
        // In a full implementation, this would support glob patterns
        facts // For now, return all facts
    }

    fn facts_to_json(
        &self,
        facts: SystemFacts,
    ) -> Result<HashMap<String, serde_json::Value>, ModuleExecutionError> {
        let json_value =
            serde_json::to_value(facts).map_err(|e| ModuleExecutionError::ExecutionFailed {
                message: format!("Failed to serialize facts: {e}"),
            })?;

        if let serde_json::Value::Object(map) = json_value {
            Ok(map.into_iter().collect())
        } else {
            Err(ModuleExecutionError::ExecutionFailed {
                message: "Facts serialization resulted in non-object".to_string(),
            })
        }
    }
}

impl Default for SetupModule {
    fn default() -> Self {
        Self::new()
    }
}
