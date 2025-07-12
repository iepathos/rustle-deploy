use super::{CopyResult, OutputStrategy};
use crate::compilation::output::error::OutputError;
use crate::compilation::{compiler::BinarySource, compiler::CompiledBinary};
use async_trait::async_trait;
use std::path::Path;
use std::time::Instant;
use tracing::debug;

#[derive(Debug, Default)]
pub struct InMemoryOutputStrategy;

impl InMemoryOutputStrategy {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl OutputStrategy for InMemoryOutputStrategy {
    async fn copy_binary(
        &self,
        binary: &CompiledBinary,
        output_path: &Path,
    ) -> Result<CopyResult, OutputError> {
        let start_time = Instant::now();

        // This strategy can handle any source type by using the binary_data field
        if binary.binary_data.is_empty() {
            return Err(OutputError::SourceNotFound {
                path: binary.binary_path.clone(),
            });
        }

        // Create output directory if needed
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Atomic write operation
        let temp_path = output_path.with_extension("tmp");
        tokio::fs::write(&temp_path, &binary.binary_data).await?;
        tokio::fs::rename(&temp_path, output_path).await?;

        // Verify copy integrity
        let copied_size = tokio::fs::metadata(output_path).await?.len();

        debug!(
            "Copied binary from memory: {} bytes -> {}",
            binary.binary_data.len(),
            output_path.display()
        );

        Ok(CopyResult {
            output_path: output_path.to_path_buf(),
            bytes_copied: copied_size,
            copy_duration: start_time.elapsed(),
            source_verified: copied_size == binary.size,
        })
    }

    fn can_handle(&self, _source: &BinarySource) -> bool {
        // Can handle any source type as fallback using binary_data
        true
    }

    fn priority(&self) -> u8 {
        10 // Lowest priority - fallback strategy
    }

    fn name(&self) -> &'static str {
        "memory"
    }
}
