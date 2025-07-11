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
    pub rustle_version: String,
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
    pub target_architecture: String,
    pub task_ids: Vec<String>,
    #[serde(with = "serde_duration")]
    pub estimated_savings: Duration,
    pub compilation_requirements: CompilationRequirements,
    // Template generation fields
    pub controller_endpoint: Option<String>,
    #[serde(with = "serde_duration_opt")]
    pub execution_timeout: Option<Duration>,
    #[serde(with = "serde_duration_opt")]
    pub report_interval: Option<Duration>,
    pub cleanup_on_completion: Option<bool>,
    pub log_level: Option<String>,
    pub max_retries: Option<u32>,
    pub static_files: Vec<StaticFileRef>,
    pub secrets: Vec<SecretRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationRequirements {
    pub modules: Vec<String>,
    pub static_files: Vec<String>,
    pub target_triple: String,
    pub optimization_level: String,
    pub features: Vec<String>,
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
