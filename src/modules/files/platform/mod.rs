//! Platform-specific file operations

#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub mod windows;

// Re-export platform-specific implementations
#[cfg(unix)]
pub use unix::*;

#[cfg(windows)]
pub use windows::*;
