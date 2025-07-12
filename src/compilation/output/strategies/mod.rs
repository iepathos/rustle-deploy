pub mod cache_strategy;
pub mod memory_strategy;
pub mod project_strategy;

pub use cache_strategy::*;
pub use memory_strategy::*;
pub use project_strategy::*;

use crate::compilation::output::error::OutputError;
use crate::compilation::{BinarySource, CompiledBinary};
use async_trait::async_trait;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug)]
pub struct CopyResult {
    pub output_path: PathBuf,
    pub bytes_copied: u64,
    pub copy_duration: Duration,
    pub source_verified: bool,
}

#[async_trait]
pub trait OutputStrategy: Send + Sync {
    async fn copy_binary(
        &self,
        binary: &CompiledBinary,
        output_path: &Path,
    ) -> Result<CopyResult, OutputError>;

    fn can_handle(&self, source: &BinarySource) -> bool;
    fn priority(&self) -> u8; // Higher = more preferred
    fn name(&self) -> &'static str;
}
