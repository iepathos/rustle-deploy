//! Archive operations module

pub mod archive;
pub mod formats;
pub mod unarchive;
pub mod utils;

pub use archive::{ArchiveArgs, ArchiveModule, ArchiveResult};
pub use formats::{ArchiveDetector, ArchiveFormat};
pub use unarchive::{UnarchiveArgs, UnarchiveModule, UnarchiveResult};
