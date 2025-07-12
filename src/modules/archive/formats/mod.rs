//! Archive format handlers

pub mod detection;
pub mod tar;
pub mod zip;

pub use detection::{ArchiveDetector, ArchiveFormat, DetectionError};
pub use tar::{TarError, TarHandler};
pub use zip::{ZipError, ZipHandler};
