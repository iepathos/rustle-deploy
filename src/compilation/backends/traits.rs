use crate::template::GeneratedTemplate;
use crate::types::compilation::{CompiledBinary, TargetSpecification};
/// Abstract compilation backend trait for rustle-deploy
use async_trait::async_trait;

#[async_trait]
pub trait CompilationBackend: Send + Sync {
    type Error: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static;
    type Config: Default + Clone + Send + Sync;

    async fn compile_binary(
        &self,
        template: &GeneratedTemplate,
        target: &TargetSpecification,
        config: &Self::Config,
    ) -> Result<CompiledBinary, Self::Error>;

    fn supports_target(&self, target: &str) -> bool;
    fn get_capabilities(&self) -> BackendCapabilities;
    fn backend_name(&self) -> &'static str;
}

#[derive(Debug, Clone)]
pub struct BackendCapabilities {
    pub supported_targets: Vec<String>,
    pub supports_cross_compilation: bool,
    pub supports_static_linking: bool,
    pub supports_lto: bool,
    pub requires_toolchain: bool,
}

impl Default for BackendCapabilities {
    fn default() -> Self {
        Self {
            supported_targets: Vec::new(),
            supports_cross_compilation: false,
            supports_static_linking: false,
            supports_lto: false,
            requires_toolchain: true,
        }
    }
}
