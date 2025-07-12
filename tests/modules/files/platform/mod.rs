//! Platform-specific tests for file operations

pub mod cross_platform_tests;

#[cfg(unix)]
pub mod unix_tests;

#[cfg(windows)]
pub mod windows_tests;
