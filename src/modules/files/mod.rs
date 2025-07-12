//! File operation modules for rustle-deploy
//!
//! This module provides essential file operations including:
//! - File attribute management (permissions, ownership, state)
//! - File copying with validation and backup
//! - File information gathering
//! - Template processing with variable substitution

pub mod copy;
pub mod file;
pub mod stat;
pub mod template;
pub mod template_engine;

// Utility modules
pub mod platform;
pub mod utils;

// Re-export main modules
pub use copy::CopyModule;
pub use file::FileModule;
pub use stat::StatModule;
pub use template::TemplateModule;

// Re-export common types
pub use copy::CopyArgs;
pub use file::{FileArgs, FileState};
pub use stat::{StatArgs, StatResult};
pub use template::TemplateArgs;
