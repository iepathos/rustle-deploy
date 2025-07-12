use crate::types::compilation::BinaryCompilation;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Main deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentPlan {
    pub metadata: DeploymentMetadata,
    pub binary_compilations: Vec<BinaryCompilation>,
    pub deployment_targets: Vec<DeploymentTarget>,
    pub deployment_strategy: DeploymentStrategy,
    pub rollback_info: Option<RollbackInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentMetadata {
    pub deployment_id: String,
    pub created_at: DateTime<Utc>,
    pub rustle_plan_version: String,
    pub execution_plan_hash: String,
    pub compiler_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentTarget {
    pub host: String,
    pub target_path: String,
    pub binary_compilation_id: String,
    pub deployment_method: DeploymentMethod,
    pub status: DeploymentStatus,
    pub deployed_at: Option<DateTime<Utc>>,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentMethod {
    Ssh,
    Scp,
    Rsync,
    Custom { command: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentStatus {
    Pending,
    Compiling,
    Compiled,
    Deploying,
    Deployed,
    Failed { error: String },
    Verified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentStrategy {
    Parallel,
    Rolling { batch_size: u32 },
    BlueGreen,
    CanaryDeployment { percentage: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackInfo {
    pub previous_deployment_id: String,
    pub rollback_strategy: RollbackStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RollbackStrategy {
    Immediate,
    Gradual { batch_size: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub controller_endpoint: Option<String>,
    #[serde(with = "serde_duration")]
    pub execution_timeout: Duration,
    #[serde(with = "serde_duration")]
    pub report_interval: Duration,
    pub cleanup_on_completion: bool,
    pub log_level: String,
    #[serde(default)]
    pub verbose: bool,
}

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
