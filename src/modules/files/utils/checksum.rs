//! File checksum calculation utilities

use md5::Md5;
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

use super::FileError;

/// Supported checksum algorithms
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ChecksumAlgorithm {
    Md5,
    Sha1,
    #[default]
    Sha256,
}

impl std::str::FromStr for ChecksumAlgorithm {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "md5" => Ok(ChecksumAlgorithm::Md5),
            "sha1" => Ok(ChecksumAlgorithm::Sha1),
            "sha256" => Ok(ChecksumAlgorithm::Sha256),
            _ => Err(format!("Unsupported checksum algorithm: {s}")),
        }
    }
}

/// Calculate file checksum using specified algorithm
pub async fn calculate_file_checksum(
    path: &Path,
    algorithm: ChecksumAlgorithm,
) -> Result<String, FileError> {
    let mut file = File::open(path).await?;
    let mut buffer = vec![0; 8192];

    match algorithm {
        ChecksumAlgorithm::Md5 => {
            let mut hasher = Md5::new();
            loop {
                let bytes_read = file.read(&mut buffer).await?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
            }
            Ok(format!("{:x}", hasher.finalize()))
        }
        ChecksumAlgorithm::Sha1 => {
            let mut hasher = Sha1::new();
            loop {
                let bytes_read = file.read(&mut buffer).await?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
            }
            Ok(format!("{:x}", hasher.finalize()))
        }
        ChecksumAlgorithm::Sha256 => {
            let mut hasher = Sha256::new();
            loop {
                let bytes_read = file.read(&mut buffer).await?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
            }
            Ok(format!("{:x}", hasher.finalize()))
        }
    }
}

/// Verify file checksum against expected value
pub async fn verify_file_checksum(
    path: &Path,
    expected: &str,
    algorithm: ChecksumAlgorithm,
) -> Result<bool, FileError> {
    let actual = calculate_file_checksum(path, algorithm).await?;
    Ok(actual.eq_ignore_ascii_case(expected))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_checksum_calculation() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut file = File::from_std(temp_file.reopen().unwrap());
        file.write_all(b"hello world").await.unwrap();
        file.flush().await.unwrap();

        // Test SHA256
        let checksum = calculate_file_checksum(temp_file.path(), ChecksumAlgorithm::Sha256)
            .await
            .unwrap();
        assert_eq!(
            checksum,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );

        // Test verification
        let is_valid = verify_file_checksum(
            temp_file.path(),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9",
            ChecksumAlgorithm::Sha256,
        )
        .await
        .unwrap();
        assert!(is_valid);
    }
}
