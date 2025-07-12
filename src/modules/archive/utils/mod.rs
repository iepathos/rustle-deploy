//! Archive utilities

pub mod compression;
pub mod extraction;

pub use compression::{CompressionError, CompressionReader, CompressionWriter};
pub use extraction::{utils, CreationOptions, CreationResult, ExtractionOptions, ExtractionResult};
