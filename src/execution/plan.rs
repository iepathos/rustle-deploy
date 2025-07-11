use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Complete execution plan from rustle-plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub metadata: ExecutionPlanMetadata,
    pub tasks: Vec<Task>,
    pub inventory: InventorySpec,
    pub strategy: ExecutionStrategy,
    pub facts_template: FactsTemplate,
    pub deployment_config: DeploymentConfig,
    pub modules: Vec<ModuleSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlanMetadata {
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub rustle_plan_version: String,
    pub plan_id: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub task_type: TaskType,
    pub module: String,
    pub args: HashMap<String, serde_json::Value>,
    pub dependencies: Vec<String>,
    pub conditions: Vec<Condition>,
    pub target_hosts: TargetSelector,
    #[serde(with = "serde_duration_opt")]
    pub timeout: Option<Duration>,
    pub retry_policy: Option<RetryPolicy>,
    pub failure_policy: FailurePolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    Command,
    Copy,
    Template,
    Package,
    Service,
    Custom { module_name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub variable: String,
    pub operator: ConditionOperator,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    Contains,
    StartsWith,
    EndsWith,
    GreaterThan,
    LessThan,
    Exists,
    NotExists,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetSelector {
    All,
    Groups(Vec<String>),
    Hosts(Vec<String>),
    Expression(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    #[serde(with = "serde_duration")]
    pub delay: Duration,
    pub backoff: BackoffStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Fixed,
    Linear,
    Exponential,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FailurePolicy {
    Abort,
    Continue,
    Rollback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventorySpec {
    pub format: InventoryFormat,
    pub source: InventorySource,
    pub groups: HashMap<String, HostGroup>,
    pub hosts: HashMap<String, Host>,
    pub variables: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InventoryFormat {
    Yaml,
    Json,
    Ini,
    Dynamic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InventorySource {
    Inline { content: String },
    File { path: String },
    Url { url: String },
    Dynamic { script: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostGroup {
    pub hosts: Vec<String>,
    pub variables: HashMap<String, serde_json::Value>,
    pub children: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    pub address: String,
    pub connection: ConnectionConfig,
    pub variables: HashMap<String, serde_json::Value>,
    pub target_triple: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub method: ConnectionMethod,
    pub username: Option<String>,
    pub password: Option<String>,
    pub key_file: Option<String>,
    pub port: Option<u16>,
    #[serde(with = "serde_duration_opt")]
    pub timeout: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionMethod {
    Ssh,
    WinRm,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactsTemplate {
    pub global_facts: Vec<String>,
    pub host_facts: Vec<String>,
    pub custom_facts: HashMap<String, FactDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactDefinition {
    pub command: String,
    pub parser: FactParser,
    #[serde(with = "serde_duration_opt")]
    pub cache_ttl: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FactParser {
    Json,
    Yaml,
    Text,
    Regex { pattern: String },
    Custom { script: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSpec {
    pub name: String,
    pub source: ModuleSource,
    pub version: Option<String>,
    pub checksum: Option<String>,
    pub dependencies: Vec<String>,
    pub static_link: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleSource {
    Builtin,
    File {
        path: String,
    },
    Git {
        repository: String,
        reference: String,
    },
    Http {
        url: String,
    },
    Registry {
        name: String,
        version: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStrategy {
    pub parallel_limit: Option<u32>,
    pub fail_fast: bool,
    pub retry_failed: bool,
    pub rollback_on_failure: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConfig {
    pub target_path: String,
    pub backup_previous: bool,
    pub verify_deployment: bool,
    pub cleanup_on_success: bool,
    #[serde(with = "serde_duration_opt")]
    pub deployment_timeout: Option<Duration>,
}

// Custom serialization for Duration fields
mod serde_duration {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
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
            Some(d) => Some(d.as_secs()).serialize(serializer),
            None => None::<u64>.serialize(serializer),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs_opt = Option::<u64>::deserialize(deserializer)?;
        Ok(secs_opt.map(Duration::from_secs))
    }
}
