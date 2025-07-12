use super::{CopyResult, OutputStrategy};
use crate::compilation::output::error::OutputError;
use crate::types::compilation::{BinarySourceType, CompiledBinary};
use async_trait::async_trait;
use std::path::Path;
use std::time::Instant;
use tracing::debug;

#[derive(Debug, Default)]
pub struct CacheOutputStrategy;

impl CacheOutputStrategy {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl OutputStrategy for CacheOutputStrategy {
    type Error = OutputError;
    async fn copy_binary(
        &self,
        binary: &CompiledBinary,
        output_path: &Path,
    ) -> Result<CopyResult, Self::Error> {
        let start_time = Instant::now();

        // For cache strategy, we write binary data directly
        debug!(
            "Cache strategy: writing {} bytes to {}",
            binary.size,
            output_path.display()
        );

        // Write binary data to output path
        match tokio::fs::write(output_path, &binary.binary_data).await {
            Ok(_) => {}
            Err(e) => {
                return Err(OutputError::CopyFailed {
                    source_path: "binary_data".into(),
                    destination: output_path.to_path_buf(),
                    message: e.to_string(),
                })
            }
        }

        // Verify copy integrity
        let copied_size = tokio::fs::metadata(output_path).await?.len();

        debug!(
            "Copied binary from memory: {} bytes to {}",
            copied_size,
            output_path.display()
        );

        Ok(CopyResult {
            output_path: output_path.to_path_buf(),
            bytes_copied: copied_size,
            copy_duration: start_time.elapsed(),
            source_verified: copied_size == binary.size,
        })
    }

    fn can_handle(&self, source_type: &BinarySourceType) -> bool {
        matches!(source_type, BinarySourceType::Cache { .. })
    }

    fn priority(&self) -> u8 {
        100 // Highest priority - cache is fastest
    }

    fn name(&self) -> &'static str {
        "cache"
    }
}
