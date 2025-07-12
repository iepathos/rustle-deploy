pub mod backends;
pub mod cache;
pub mod capabilities;
pub mod compiler;
pub mod optimizer;
pub mod output;
pub mod target_detection;
pub mod toolchain;
pub mod zero_infra;
pub mod zigbuild;

// Public API - only export what external modules should use
pub use backends::{BackendRegistry, CompilationConfig as BackendConfig};
pub use cache::*;
pub use capabilities::*;
pub use compiler::{BinaryCompiler, CompilerConfig};
pub use optimizer::*;
pub use output::*;
pub use target_detection::*;
pub use toolchain::*;
pub use zero_infra::*;

// Re-export canonical types from the types module
pub use crate::types::compilation::{
    BinarySourceInfo, BinarySourceType, CompilationOptions, CompiledBinary, OptimizationLevel,
    TargetSpecification,
};
