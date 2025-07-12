//! Platform-specific fact collectors

use crate::modules::system::facts::FactError;
use async_trait::async_trait;
use std::collections::HashMap;

#[cfg(target_os = "freebsd")]
pub mod freebsd;
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod unix_common;
#[cfg(target_os = "windows")]
pub mod windows;

#[async_trait]
pub trait PlatformFactCollector: Send + Sync {
    async fn collect_platform_facts(&self)
        -> Result<HashMap<String, serde_json::Value>, FactError>;
    async fn collect_virtualization_facts(
        &self,
    ) -> Result<HashMap<String, serde_json::Value>, FactError>;
}
