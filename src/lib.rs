//! Rustle Deploy - Binary compiler and deployment manager
//!
//! This crate provides functionality for compiling execution plans into optimized
//! target binaries with embedded execution data and deploying them to remote hosts.

#![recursion_limit = "256"]

pub mod binary;
// pub mod cli;  // Temporarily disabled to fix compilation
pub mod compilation;
pub mod compiler;
pub mod deploy;
pub mod execution;
pub mod inventory;
pub mod modules;
pub mod runtime;
pub mod template;
pub mod types;

// pub use compilation::{
//     BinaryCompiler, CompilationCache, CompilerConfig, TargetDetector, TargetSpecification,
// };
pub use deploy::DeploymentManager;
pub use inventory::*;
pub use types::*;
