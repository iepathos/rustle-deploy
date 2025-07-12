//! Comprehensive test suite for file operations modules
//!
//! This test suite provides:
//! - Integration tests for real file system operations
//! - Property-based tests for edge cases and invariants
//! - Platform-specific behavior validation
//! - Performance benchmarks
//! - End-to-end workflow testing

pub mod helpers;
pub mod integration;
pub mod platform;
pub mod property;

// Re-export common test utilities
pub use helpers::{
    assertions::*,
    builders::{CopyTestBuilder, FileTestBuilder, StatTestBuilder, TemplateTestBuilder},
    environment::TestEnvironment,
    fixtures::TestFixtures,
};
