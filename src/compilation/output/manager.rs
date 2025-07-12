use crate::compilation::output::error::OutputError;
use crate::compilation::output::strategies::{
    CacheOutputStrategy, CopyResult, InMemoryOutputStrategy, OutputStrategy, ProjectOutputStrategy,
};
use crate::compilation::{CompilationCache, CompiledBinary};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

pub struct BinaryOutputManager {
    #[allow(dead_code)]
    cache: CompilationCache,
    output_strategies: Vec<Box<dyn OutputStrategy>>,
}

impl BinaryOutputManager {
    pub fn new(cache: CompilationCache) -> Self {
        let strategies: Vec<Box<dyn OutputStrategy>> = vec![
            Box::new(CacheOutputStrategy::new()),
            Box::new(ProjectOutputStrategy::new()),
            Box::new(InMemoryOutputStrategy::new()),
        ];

        Self {
            cache,
            output_strategies: strategies,
        }
    }

    pub async fn copy_to_output(
        &self,
        binary: &CompiledBinary,
        output_path: &Path,
    ) -> Result<CopyResult, OutputError> {
        // Adjust output path for target platform
        let adjusted_output_path =
            self.adjust_output_path_for_target(output_path, &binary.target_triple);

        // Sort strategies by priority and compatibility
        let mut compatible_strategies: Vec<_> = self
            .output_strategies
            .iter()
            .filter(|s| s.can_handle(&binary.effective_source))
            .collect();
        compatible_strategies.sort_by_key(|s| std::cmp::Reverse(s.priority()));

        let mut last_error = None;

        for strategy in compatible_strategies {
            match strategy.copy_binary(binary, &adjusted_output_path).await {
                Ok(result) => {
                    info!(
                        "Binary copied via {} strategy: {} bytes in {:?}",
                        strategy.name(),
                        result.bytes_copied,
                        result.copy_duration
                    );
                    return Ok(result);
                }
                Err(e) => {
                    debug!("Strategy {} failed: {}", strategy.name(), e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or(OutputError::NoCompatibleStrategy))
    }

    fn adjust_output_path_for_target(&self, base_path: &Path, target_triple: &str) -> PathBuf {
        let mut path = base_path.to_path_buf();

        // Add .exe extension for Windows targets
        if target_triple.contains("windows") {
            path.set_extension("exe");
        }

        path
    }
}
