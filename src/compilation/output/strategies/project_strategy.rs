use super::{CopyResult, OutputStrategy};
use crate::compilation::output::error::OutputError;
use crate::compilation::{compiler::BinarySource, compiler::CompiledBinary};
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
    async fn copy_binary(
        &self,
        binary: &CompiledBinary,
        output_path: &Path,
    ) -> Result<CopyResult, OutputError> {
        let start_time = Instant::now();

        let project_path = &binary.binary_path;

        // Verify project binary exists and is accessible
        if !project_path.exists() {
            return Err(OutputError::SourceNotFound {
                path: project_path.clone(),
            });
        }

        // Create output directory if needed
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Atomic copy operation
        let temp_path = output_path.with_extension("tmp");
        tokio::fs::copy(project_path, &temp_path).await?;
        tokio::fs::rename(&temp_path, output_path).await?;

        // Verify copy integrity
        let copied_size = tokio::fs::metadata(output_path).await?.len();

        debug!(
            "Copied binary from project: {} -> {} ({} bytes)",
            project_path.display(),
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

    fn can_handle(&self, source: &BinarySource) -> bool {
        matches!(source, BinarySource::FreshCompilation { .. })
    }

    fn priority(&self) -> u8 {
        80 // Lower priority than cache
    }

    fn name(&self) -> &'static str {
        "project"
    }
}
