use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Parsed inventory with hosts and groups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedInventory {
    pub hosts: HashMap<String, InventoryHost>,
    pub groups: HashMap<String, InventoryGroup>,
    pub global_vars: HashMap<String, serde_json::Value>,
    pub metadata: InventoryMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryHost {
    pub name: String,
    pub address: Option<String>,
    pub connection: ConnectionConfig,
    pub variables: HashMap<String, serde_json::Value>,
    pub groups: Vec<String>,
    pub target_triple: Option<String>,
    pub architecture: Option<String>,
    pub operating_system: Option<String>,
    pub platform: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryGroup {
    pub name: String,
    pub hosts: Vec<String>,
    pub children: Vec<String>,
    pub variables: HashMap<String, serde_json::Value>,
    pub parent_groups: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryMetadata {
    pub format: InventoryFormat,
    pub source: String,
    pub parsed_at: DateTime<Utc>,
    pub host_count: usize,
    pub group_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InventoryFormat {
    Yaml,
    Json,
    Ini,
    Dynamic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub method: ConnectionMethod,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub private_key_file: Option<String>,
    #[serde(with = "serde_duration_opt")]
    pub timeout: Option<Duration>,
    pub ssh_args: Option<String>,
    pub winrm_transport: Option<WinRmTransport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionMethod {
    Ssh,
    WinRm,
    Local,
    Docker,
    Podman,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WinRmTransport {
    Http,
    Https,
    Kerberos,
    Ntlm,
}

/// Host information detection
#[derive(Debug, Clone)]
pub struct HostInfo {
    pub architecture: String,
    pub operating_system: String,
    pub platform: String,
    pub kernel_version: String,
    pub target_triple: String,
    pub capabilities: Vec<String>,
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
