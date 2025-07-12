//! Compression utilities for archive operations

use bzip2::{read::BzDecoder, write::BzEncoder};
use flate2::{read::GzDecoder, write::GzEncoder, Compression as GzCompression};
use std::io::{Read, Write};
use xz2::{read::XzDecoder, write::XzEncoder};

#[derive(Debug, thiserror::Error)]
pub enum CompressionError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Compression error: {0}")]
    Compression(String),
}

/// Wrapper for compression readers
pub enum CompressionReader<R: Read> {
    Gzip(GzDecoder<R>),
    Bzip2(BzDecoder<R>),
    Xz(XzDecoder<R>),
}

impl<R: Read> CompressionReader<R> {
    pub fn new_gzip(reader: R) -> Result<Self, CompressionError> {
        Ok(CompressionReader::Gzip(GzDecoder::new(reader)))
    }

    pub fn new_bzip2(reader: R) -> Result<Self, CompressionError> {
        Ok(CompressionReader::Bzip2(BzDecoder::new(reader)))
    }

    pub fn new_xz(reader: R) -> Result<Self, CompressionError> {
        Ok(CompressionReader::Xz(XzDecoder::new(reader)))
    }
}

impl<R: Read> Read for CompressionReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            CompressionReader::Gzip(decoder) => decoder.read(buf),
            CompressionReader::Bzip2(decoder) => decoder.read(buf),
            CompressionReader::Xz(decoder) => decoder.read(buf),
        }
    }
}

/// Wrapper for compression writers
pub enum CompressionWriter<W: Write> {
    Gzip(GzEncoder<W>),
    Bzip2(BzEncoder<W>),
    Xz(XzEncoder<W>),
}

impl<W: Write> CompressionWriter<W> {
    pub fn new_gzip(writer: W, level: Option<u8>) -> Result<Self, CompressionError> {
        let compression_level = level.map(|l| if l > 9 { 9 } else { l }).unwrap_or(6);

        let compression = GzCompression::new(compression_level as u32);
        Ok(CompressionWriter::Gzip(GzEncoder::new(writer, compression)))
    }

    pub fn new_bzip2(writer: W, level: Option<u8>) -> Result<Self, CompressionError> {
        let compression_level = level
            .map(|l| {
                if l > 9 {
                    9
                } else if l < 1 {
                    1
                } else {
                    l
                }
            })
            .unwrap_or(6);

        let compression = bzip2::Compression::new(compression_level as u32);
        Ok(CompressionWriter::Bzip2(BzEncoder::new(
            writer,
            compression,
        )))
    }

    pub fn new_xz(writer: W, level: Option<u8>) -> Result<Self, CompressionError> {
        let compression_level = level.map(|l| if l > 9 { 9 } else { l }).unwrap_or(6);

        Ok(CompressionWriter::Xz(XzEncoder::new(
            writer,
            compression_level as u32,
        )))
    }

    pub fn finish(self) -> Result<W, CompressionError> {
        match self {
            CompressionWriter::Gzip(encoder) => encoder.finish().map_err(CompressionError::Io),
            CompressionWriter::Bzip2(encoder) => encoder.finish().map_err(CompressionError::Io),
            CompressionWriter::Xz(encoder) => encoder.finish().map_err(CompressionError::Io),
        }
    }
}

impl<W: Write> Write for CompressionWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            CompressionWriter::Gzip(encoder) => encoder.write(buf),
            CompressionWriter::Bzip2(encoder) => encoder.write(buf),
            CompressionWriter::Xz(encoder) => encoder.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            CompressionWriter::Gzip(encoder) => encoder.flush(),
            CompressionWriter::Bzip2(encoder) => encoder.flush(),
            CompressionWriter::Xz(encoder) => encoder.flush(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_gzip_compression_roundtrip() {
        let original_data = b"Hello, world! This is a test string for compression.";

        // Compress
        let mut compressed = Vec::new();
        {
            let mut encoder = CompressionWriter::new_gzip(&mut compressed, Some(6)).unwrap();
            encoder.write_all(original_data).unwrap();
            encoder.finish().unwrap();
        }

        // Decompress
        let mut decompressed = Vec::new();
        {
            let cursor = Cursor::new(&compressed);
            let mut decoder = CompressionReader::new_gzip(cursor).unwrap();
            decoder.read_to_end(&mut decompressed).unwrap();
        }

        assert_eq!(original_data, decompressed.as_slice());
    }

    #[test]
    fn test_bzip2_compression_roundtrip() {
        let original_data = b"Hello, world! This is a test string for compression.";

        // Compress
        let mut compressed = Vec::new();
        {
            let mut encoder = CompressionWriter::new_bzip2(&mut compressed, Some(6)).unwrap();
            encoder.write_all(original_data).unwrap();
            encoder.finish().unwrap();
        }

        // Decompress
        let mut decompressed = Vec::new();
        {
            let cursor = Cursor::new(&compressed);
            let mut decoder = CompressionReader::new_bzip2(cursor).unwrap();
            decoder.read_to_end(&mut decompressed).unwrap();
        }

        assert_eq!(original_data, decompressed.as_slice());
    }

    #[test]
    fn test_xz_compression_roundtrip() {
        let original_data = b"Hello, world! This is a test string for compression.";

        // Compress
        let mut compressed = Vec::new();
        {
            let mut encoder = CompressionWriter::new_xz(&mut compressed, Some(6)).unwrap();
            encoder.write_all(original_data).unwrap();
            encoder.finish().unwrap();
        }

        // Decompress
        let mut decompressed = Vec::new();
        {
            let cursor = Cursor::new(&compressed);
            let mut decoder = CompressionReader::new_xz(cursor).unwrap();
            decoder.read_to_end(&mut decompressed).unwrap();
        }

        assert_eq!(original_data, decompressed.as_slice());
    }
}
