//! Core execution modules for Ansible-compatible binary deployments

pub mod core;
pub mod error;
pub mod interface;
pub mod registry;
pub mod system;

// Re-export commonly used types
pub use error::*;
pub use interface::*;
pub use registry::ModuleRegistry;
