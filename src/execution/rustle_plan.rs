use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use super::plan::ExecutionStrategy;

/// Rustle-plan compatible execution plan format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustlePlanOutput {
    pub metadata: RustlePlanMetadata,
    pub plays: Vec<PlayPlan>,
    pub binary_deployments: Vec<BinaryDeploymentPlan>,
    pub total_tasks: u32,
    #[serde(with = "serde_duration_opt")]
    pub estimated_duration: Option<Duration>,
    #[serde(with = "serde_duration_opt")]
    pub estimated_compilation_time: Option<Duration>,
    pub parallelism_score: f32,
    pub network_efficiency_score: f32,
    pub hosts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustlePlanMetadata {
    pub created_at: DateTime<Utc>,
    pub rustle_plan_version: String,
    pub playbook_hash: String,
    pub inventory_hash: String,
    pub planning_options: PlanningOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningOptions {
    pub limit: Option<String>,
    pub tags: Vec<String>,
    pub skip_tags: Vec<String>,
    pub check_mode: bool,
    pub diff_mode: bool,
    pub forks: u32,
    pub serial: Option<u32>,
    pub strategy: ExecutionStrategy,
    pub binary_threshold: u32,
    pub force_binary: bool,
    pub force_ssh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayPlan {
    pub play_id: String,
    pub name: String,
    pub strategy: ExecutionStrategy,
    pub serial: Option<u32>,
    pub hosts: Vec<String>,
    pub batches: Vec<TaskBatch>,
    pub handlers: Vec<HandlerDefinition>,
    #[serde(with = "serde_duration_opt")]
    pub estimated_duration: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBatch {
    pub batch_id: String,
    pub hosts: Vec<String>,
    pub tasks: Vec<TaskPlan>,
    pub parallel_groups: Vec<ParallelGroup>,
    pub dependencies: Vec<String>,
    #[serde(with = "serde_duration_opt")]
    pub estimated_duration: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    pub task_id: String,
    pub name: String,
    pub module: String,
    pub args: HashMap<String, serde_json::Value>,
    pub hosts: Vec<String>,
    pub dependencies: Vec<String>,
    pub conditions: Vec<TaskCondition>,
    pub tags: Vec<String>,
    pub notify: Vec<String>,
    pub execution_order: u32,
    pub can_run_parallel: bool,
    #[serde(with = "serde_duration")]
    pub estimated_duration: Duration,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskCondition {
    Tag { tags: Vec<String> },
    When { expression: String },
    Skip { condition: String },
    Only { hosts: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelGroup {
    pub group_id: String,
    pub tasks: Vec<String>,
    pub max_parallelism: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerDefinition {
    pub handler_id: String,
    pub name: String,
    pub module: String,
    pub args: HashMap<String, serde_json::Value>,
    pub conditions: Vec<TaskCondition>,
    pub execution_order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryDeploymentPlan {
    pub deployment_id: String,
    pub target_hosts: Vec<String>,
    #[serde(default)]
    pub binary_name: String,
    #[serde(default)]
    pub tasks: Vec<String>,
    #[serde(default)]
    pub modules: Vec<String>,
    #[serde(default)]
    pub embedded_data: EmbeddedData,
    #[serde(default)]
    pub execution_mode: ExecutionMode,
    #[serde(default)]
    pub estimated_size: u64,
    pub compilation_requirements: CompilationRequirements,

    // Legacy fields (deprecated but maintained for compatibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_architecture: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_duration_opt_legacy"
    )]
    pub estimated_savings: Option<Duration>,

    // Existing template generation fields (unchanged)
    #[serde(default)]
    pub controller_endpoint: Option<String>,
    #[serde(default, with = "serde_duration_opt")]
    pub execution_timeout: Option<Duration>,
    #[serde(default, with = "serde_duration_opt")]
    pub report_interval: Option<Duration>,
    #[serde(default)]
    pub cleanup_on_completion: Option<bool>,
    #[serde(default)]
    pub log_level: Option<String>,
    #[serde(default)]
    pub max_retries: Option<u32>,
    #[serde(default)]
    pub static_files: Vec<StaticFileRef>,
    #[serde(default)]
    pub secrets: Vec<SecretRef>,
    #[serde(default)]
    pub verbose: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationRequirements {
    // New format fields
    #[serde(default)]
    pub target_arch: String,
    #[serde(default)]
    pub target_os: String,
    #[serde(default)]
    pub rust_version: String,
    #[serde(default)]
    pub cross_compilation: bool,
    #[serde(default)]
    pub static_linking: bool,

    // Legacy fields (deprecated but maintained for compatibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modules: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub static_files: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_triple: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimization_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticFileRef {
    pub source_path: String,
    pub target_path: String,
    pub permissions: Option<u32>,
    pub compress: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretRef {
    pub key: String,
    pub source: SecretSource,
    pub target_env_var: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecretSource {
    File { path: String },
    Environment { var: String },
    Vault { path: String, key: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmbeddedData {
    #[serde(default)]
    pub execution_plan: String,
    #[serde(default)]
    pub static_files: Vec<EmbeddedStaticFile>,
    #[serde(default)]
    pub variables: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub facts_required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedStaticFile {
    pub src_path: String,
    pub dest_path: String,
    pub checksum: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ExecutionMode {
    #[default]
    Controller,
    Standalone,
    Hybrid,
}

#[derive(Debug, Clone)]
pub enum BinaryCompatibility {
    FullyCompatible,
    PartiallyCompatible { limitations: Vec<String> },
    Incompatible { reasons: Vec<String> },
}

// Custom serialization for Duration fields (reusing from plan.rs)
mod serde_duration {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let secs = duration.as_secs();
        let nanos = duration.subsec_nanos();
        serde_json::json!({
            "secs": secs,
            "nanos": nanos
        })
        .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct DurationHelper {
            secs: u64,
            nanos: u32,
        }

        let helper = DurationHelper::deserialize(deserializer)?;
        Ok(Duration::new(helper.secs, helper.nanos))
    }
}

mod serde_duration_opt {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match duration {
            Some(d) => {
                let secs = d.as_secs();
                let nanos = d.subsec_nanos();
                Some(serde_json::json!({
                    "secs": secs,
                    "nanos": nanos
                }))
                .serialize(serializer)
            }
            None => None::<serde_json::Value>.serialize(serializer),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct DurationHelper {
            secs: u64,
            nanos: u32,
        }

        let helper_opt = Option::<DurationHelper>::deserialize(deserializer)?;
        Ok(helper_opt.map(|helper| Duration::new(helper.secs, helper.nanos)))
    }
}

mod serde_duration_opt_legacy {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match duration {
            Some(d) => {
                let secs = d.as_secs();
                let nanos = d.subsec_nanos();
                Some(serde_json::json!({
                    "secs": secs,
                    "nanos": nanos
                }))
                .serialize(serializer)
            }
            None => None::<serde_json::Value>.serialize(serializer),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct DurationHelper {
            secs: u64,
            nanos: u32,
        }

        let helper_opt = Option::<DurationHelper>::deserialize(deserializer)?;
        Ok(helper_opt.map(|helper| Duration::new(helper.secs, helper.nanos)))
    }
}

impl Default for CompilationRequirements {
    fn default() -> Self {
        Self {
            target_arch: "x86_64".to_string(),
            target_os: "linux".to_string(),
            rust_version: "1.70.0".to_string(),
            cross_compilation: false,
            static_linking: true,
            modules: None,
            static_files: None,
            target_triple: None,
            optimization_level: None,
            features: None,
        }
    }
}

impl Default for BinaryDeploymentPlan {
    fn default() -> Self {
        Self {
            deployment_id: "default".to_string(),
            target_hosts: vec![],
            binary_name: String::new(),
            tasks: vec![],
            modules: vec![],
            embedded_data: EmbeddedData::default(),
            execution_mode: ExecutionMode::default(),
            estimated_size: 0,
            compilation_requirements: CompilationRequirements::default(),
            task_ids: None,
            target_architecture: None,
            estimated_savings: None,
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
}

// Migration helper methods
impl BinaryDeploymentPlan {
    /// Convert legacy task_ids to new tasks format
    pub fn migrate_task_ids(&mut self) {
        if let Some(task_ids) = &self.task_ids {
            if self.tasks.is_empty() {
                self.tasks = task_ids.clone();
            }
        }
    }

    /// Extract target architecture from legacy or new format
    pub fn get_target_architecture(&self) -> String {
        if !self.compilation_requirements.target_arch.is_empty() {
            format!(
                "{}-{}",
                self.compilation_requirements.target_arch, self.compilation_requirements.target_os
            )
        } else if let Some(arch) = &self.target_architecture {
            arch.clone()
        } else {
            "unknown".to_string()
        }
    }

    /// Parse embedded execution plan as JSON
    pub fn parse_execution_plan(&self) -> Result<serde_json::Value, serde_json::Error> {
        if self.embedded_data.execution_plan.is_empty() {
            Ok(serde_json::json!({}))
        } else {
            serde_json::from_str(&self.embedded_data.execution_plan)
        }
    }

    /// Migrate from legacy format to new format
    pub fn migrate_from_legacy(&mut self) {
        // Migrate task_ids to tasks
        self.migrate_task_ids();

        // Set binary_name if not present
        if self.binary_name.is_empty() {
            self.binary_name = format!("rustle-runner-{}", self.deployment_id);
        }

        // Migrate compilation requirements
        self.compilation_requirements.migrate_from_legacy();
    }
}

impl CompilationRequirements {
    /// Create from legacy format
    pub fn from_legacy(
        modules: Vec<String>,
        target_triple: String,
        optimization_level: String,
        features: Vec<String>,
    ) -> Self {
        let (arch, os) = Self::parse_target_triple(&target_triple);

        Self {
            target_arch: arch,
            target_os: os,
            rust_version: "1.70.0".to_string(),
            cross_compilation: false,
            static_linking: true,
            modules: Some(modules),
            static_files: None,
            target_triple: Some(target_triple),
            optimization_level: Some(optimization_level),
            features: Some(features),
        }
    }

    /// Migrate from legacy format to new format
    pub fn migrate_from_legacy(&mut self) {
        // If new format fields are empty but legacy fields exist, migrate
        if self.target_arch.is_empty() || self.target_os.is_empty() {
            if let Some(ref target_triple) = self.target_triple {
                let (arch, os) = Self::parse_target_triple(target_triple);
                if self.target_arch.is_empty() {
                    self.target_arch = arch;
                }
                if self.target_os.is_empty() {
                    self.target_os = os;
                }
            }
        }

        // Set default rust_version if not present
        if self.rust_version.is_empty() {
            self.rust_version = "1.70.0".to_string();
        }
    }

    fn parse_target_triple(triple: &str) -> (String, String) {
        let parts: Vec<&str> = triple.split('-').collect();
        if parts.len() >= 3 {
            (parts[0].to_string(), parts[2].to_string())
        } else {
            ("x86_64".to_string(), "linux".to_string())
        }
    }
}
