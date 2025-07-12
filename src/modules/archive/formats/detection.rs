//! Archive format detection using magic bytes and file extensions

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum ArchiveFormat {
    Tar,
    TarGz,
    TarBz2,
    TarXz,
    Zip,
    SevenZ,
    Rar,
    Auto,
}

#[derive(Debug, thiserror::Error)]
pub enum DetectionError {
    #[error("IO error during format detection: {0}")]
    Io(#[from] std::io::Error),
    #[error("Unknown archive format")]
    UnknownFormat,
    #[error("Unsupported archive format")]
    UnsupportedFormat,
}

pub struct ArchiveDetector;

impl ArchiveDetector {
    /// Detect archive format from file extension
    pub fn detect_from_extension(path: &Path) -> Result<ArchiveFormat, DetectionError> {
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();

        if filename.ends_with(".tar.gz") || filename.ends_with(".tgz") {
            Ok(ArchiveFormat::TarGz)
        } else if filename.ends_with(".tar.bz2") || filename.ends_with(".tbz2") {
            Ok(ArchiveFormat::TarBz2)
        } else if filename.ends_with(".tar.xz") || filename.ends_with(".txz") {
            Ok(ArchiveFormat::TarXz)
        } else if filename.ends_with(".tar") {
            Ok(ArchiveFormat::Tar)
        } else if filename.ends_with(".zip") {
            Ok(ArchiveFormat::Zip)
        } else if filename.ends_with(".7z") {
            Ok(ArchiveFormat::SevenZ)
        } else if filename.ends_with(".rar") {
            Ok(ArchiveFormat::Rar)
        } else {
            Err(DetectionError::UnknownFormat)
        }
    }

    /// Detect archive format from magic bytes
    pub fn detect_from_magic_bytes<R: Read + Seek>(
        reader: &mut R,
    ) -> Result<ArchiveFormat, DetectionError> {
        let mut buffer = [0u8; 512];
        let start_pos = reader.stream_position()?;

        // Try to read the buffer, but don't fail if we can't read the full amount
        let bytes_read = reader.read(&mut buffer)?;
        reader.seek(SeekFrom::Start(start_pos))?;

        if bytes_read == 0 {
            return Err(DetectionError::UnknownFormat);
        }

        // Check magic bytes
        if buffer.starts_with(b"PK\x03\x04") || buffer.starts_with(b"PK\x05\x06") {
            return Ok(ArchiveFormat::Zip);
        }

        if buffer.starts_with(&[0x1f, 0x8b]) {
            return Ok(ArchiveFormat::TarGz);
        }

        if buffer.starts_with(b"BZh") {
            return Ok(ArchiveFormat::TarBz2);
        }

        if buffer.starts_with(&[0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00]) {
            return Ok(ArchiveFormat::TarXz);
        }

        if buffer[257..262] == *b"ustar" {
            return Ok(ArchiveFormat::Tar);
        }

        if buffer.starts_with(b"7z\xbc\xaf\x27\x1c") {
            return Ok(ArchiveFormat::SevenZ);
        }

        if buffer.starts_with(b"Rar!\x1a\x07\x00") || buffer.starts_with(b"Rar!\x1a\x07\x01\x00") {
            return Ok(ArchiveFormat::Rar);
        }

        Err(DetectionError::UnknownFormat)
    }

    /// Auto-detect format using both extension and magic bytes
    pub fn auto_detect<R: Read + Seek>(
        path: &Path,
        reader: &mut R,
    ) -> Result<ArchiveFormat, DetectionError> {
        // First try extension-based detection
        if let Ok(format) = Self::detect_from_extension(path) {
            return Ok(format);
        }

        // Fall back to magic byte detection
        Self::detect_from_magic_bytes(reader)
    }

    /// Check if format is supported for extraction
    pub fn is_extraction_supported(format: &ArchiveFormat) -> bool {
        matches!(
            format,
            ArchiveFormat::Tar
                | ArchiveFormat::TarGz
                | ArchiveFormat::TarBz2
                | ArchiveFormat::TarXz
                | ArchiveFormat::Zip
        )
    }

    /// Check if format is supported for creation
    pub fn is_creation_supported(format: &ArchiveFormat) -> bool {
        matches!(
            format,
            ArchiveFormat::Tar
                | ArchiveFormat::TarGz
                | ArchiveFormat::TarBz2
                | ArchiveFormat::TarXz
                | ArchiveFormat::Zip
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_extension_detection() {
        assert_eq!(
            ArchiveDetector::detect_from_extension(Path::new("test.tar.gz")).unwrap(),
            ArchiveFormat::TarGz
        );
        assert_eq!(
            ArchiveDetector::detect_from_extension(Path::new("test.zip")).unwrap(),
            ArchiveFormat::Zip
        );
        assert_eq!(
            ArchiveDetector::detect_from_extension(Path::new("test.tar")).unwrap(),
            ArchiveFormat::Tar
        );
    }

    #[test]
    fn test_magic_byte_detection() {
        // Test ZIP magic bytes
        let zip_magic = b"PK\x03\x04test data";
        let mut cursor = Cursor::new(zip_magic);
        assert_eq!(
            ArchiveDetector::detect_from_magic_bytes(&mut cursor).unwrap(),
            ArchiveFormat::Zip
        );

        // Test gzip magic bytes
        let gzip_magic = [0x1f, 0x8b, 0x08, 0x00];
        let mut cursor = Cursor::new(gzip_magic);
        assert_eq!(
            ArchiveDetector::detect_from_magic_bytes(&mut cursor).unwrap(),
            ArchiveFormat::TarGz
        );
    }

    #[test]
    fn test_format_support() {
        assert!(ArchiveDetector::is_extraction_supported(
            &ArchiveFormat::Tar
        ));
        assert!(ArchiveDetector::is_extraction_supported(
            &ArchiveFormat::Zip
        ));
        assert!(!ArchiveDetector::is_extraction_supported(
            &ArchiveFormat::Rar
        ));

        assert!(ArchiveDetector::is_creation_supported(&ArchiveFormat::Tar));
        assert!(ArchiveDetector::is_creation_supported(&ArchiveFormat::Zip));
        assert!(!ArchiveDetector::is_creation_supported(
            &ArchiveFormat::SevenZ
        ));
    }
}
