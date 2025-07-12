//! Core execution modules for Ansible-compatible binary deployments

pub mod ast_parser;
pub mod cache;
pub mod compiler;
pub mod core;
pub mod error;
pub mod files;
pub mod interface;
pub mod loader;
pub mod registry;
pub mod resolver;
pub mod system;
pub mod validator;

// Re-export commonly used types
pub use cache::ModuleCache;
pub use compiler::CodeGenerator;
pub use error::*;
pub use files::{CopyModule, FileModule, StatModule, TemplateModule};
pub use interface::*;
pub use loader::{CompiledModule, LoadedModule, ModuleCompiler};
pub use registry::ModuleRegistry;
pub use resolver::{ModuleSourceCode, ModuleSourceResolver};
pub use validator::{ModuleValidator, ValidationResult};
