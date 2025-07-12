use super::{CopyResult, OutputStrategy};
use crate::compilation::output::error::OutputError;
use crate::types::compilation::{BinarySourceType, CompiledBinary};
use async_trait::async_trait;
use std::path::Path;
use std::time::Instant;
use tracing::debug;

#[derive(Debug, Default)]
pub struct ProjectOutputStrategy;

impl ProjectOutputStrategy {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl OutputStrategy for ProjectOutputStrategy {
    type Error = OutputError;
    async fn copy_binary(
        &self,
        binary: &CompiledBinary,
        output_path: &Path,
    ) -> Result<CopyResult, Self::Error> {
        let start_time = Instant::now();

        // Create output directory if needed
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

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
            "Copied binary from project data to {} ({} bytes)",
            output_path.display(),
            copied_size
        );

        Ok(CopyResult {
            output_path: output_path.to_path_buf(),
            bytes_copied: copied_size,
            copy_duration: start_time.elapsed(),
            source_verified: copied_size == binary.size,
        })
    }

    fn can_handle(&self, source_type: &BinarySourceType) -> bool {
        matches!(source_type, BinarySourceType::FreshCompilation { .. })
    }

    fn priority(&self) -> u8 {
        80 // Lower priority than cache
    }

    fn name(&self) -> &'static str {
        "project"
    }
}
