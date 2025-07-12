pub mod cargo;
pub mod traits;
pub mod zigbuild;

pub use traits::{BackendCapabilities, CompilationBackend};

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

/// Type alias for unified backend trait object
type BackendRef = Arc<dyn CompilationBackend<Error = anyhow::Error, Config = serde_json::Value>>;

/// Registry for managing compilation backends
#[derive(Default)]
pub struct BackendRegistry {
    backends: HashMap<String, BackendRef>,
}

impl BackendRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<B>(&mut self, backend: B) -> Result<()>
    where
        B: CompilationBackend<Error = anyhow::Error, Config = serde_json::Value> + 'static,
    {
        let name = backend.backend_name().to_string();
        self.backends.insert(name, Arc::new(backend));
        Ok(())
    }

    pub fn get_backend(&self, name: &str) -> Option<BackendRef> {
        self.backends.get(name).cloned()
    }

    pub fn select_backend_for_target(&self, target: &str) -> Option<BackendRef> {
        self.backends
            .values()
            .find(|backend| backend.supports_target(target))
            .cloned()
    }

    pub fn list_backends(&self) -> Vec<String> {
        self.backends.keys().cloned().collect()
    }

    pub fn create_default() -> Result<Self> {
        let mut registry = Self::new();

        // Register default backends
        registry.register(cargo::CargoBackend::new())?;
        registry.register(zigbuild::ZigBuildBackend::new())?;

        Ok(registry)
    }
}

/// Configuration for compilation system
#[derive(Debug, Clone)]
pub struct CompilationConfig {
    pub preferred_backend: Option<String>,
    pub fallback_enabled: bool,
    pub parallel_compilation: bool,
    pub cache_enabled: bool,
}

impl Default for CompilationConfig {
    fn default() -> Self {
        Self {
            preferred_backend: None,
            fallback_enabled: true,
            parallel_compilation: true,
            cache_enabled: true,
        }
    }
}
