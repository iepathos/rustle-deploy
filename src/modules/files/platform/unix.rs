//! Unix-specific file operations

use std::path::Path;

use crate::modules::files::utils::FileError;

/// Create a symbolic link (Unix)
pub async fn create_symlink(src: &Path, dest: &Path) -> Result<(), FileError> {
    std::os::unix::fs::symlink(src, dest)?;
    Ok(())
}

/// Create a hard link (Unix)
pub async fn create_hardlink(src: &Path, dest: &Path) -> Result<(), FileError> {
    std::fs::hard_link(src, dest)?;
    Ok(())
}

/// Get extended file attributes (Unix)
pub async fn get_extended_attributes(
    path: &Path,
) -> Result<std::collections::HashMap<String, Vec<u8>>, FileError> {
    // This would implement extended attributes support
    // For now, return empty map
    Ok(std::collections::HashMap::new())
}

/// Set extended file attributes (Unix)
pub async fn set_extended_attribute(
    path: &Path,
    name: &str,
    value: &[u8],
) -> Result<(), FileError> {
    // This would implement extended attributes support
    // For now, this is a no-op
    Ok(())
}

/// Check if file system supports certain features
pub struct FileSystemCapabilities {
    pub supports_symlinks: bool,
    pub supports_hardlinks: bool,
    pub supports_permissions: bool,
    pub supports_ownership: bool,
    pub supports_extended_attributes: bool,
}

impl FileSystemCapabilities {
    pub fn detect_for_path(_path: &Path) -> Self {
        FileSystemCapabilities {
            supports_symlinks: true,
            supports_hardlinks: true,
            supports_permissions: true,
            supports_ownership: true,
            supports_extended_attributes: true, // Most Unix filesystems do
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_create_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let src_path = temp_dir.path().join("source.txt");
        let link_path = temp_dir.path().join("link.txt");

        // Create source file
        let mut file = File::create(&src_path).await.unwrap();
        file.write_all(b"test content").await.unwrap();
        file.flush().await.unwrap();

        // Create symlink
        create_symlink(&src_path, &link_path).await.unwrap();

        // Verify link exists and points to source
        assert!(link_path.exists());
        let content = tokio::fs::read_to_string(&link_path).await.unwrap();
        assert_eq!(content, "test content");
    }

    #[tokio::test]
    async fn test_create_hardlink() {
        let temp_dir = TempDir::new().unwrap();
        let src_path = temp_dir.path().join("source.txt");
        let link_path = temp_dir.path().join("hardlink.txt");

        // Create source file
        let mut file = File::create(&src_path).await.unwrap();
        file.write_all(b"test content").await.unwrap();
        file.flush().await.unwrap();

        // Create hard link
        create_hardlink(&src_path, &link_path).await.unwrap();

        // Verify both files exist and have same content
        assert!(link_path.exists());
        let content = tokio::fs::read_to_string(&link_path).await.unwrap();
        assert_eq!(content, "test content");

        // Verify they're the same file (same inode)
        let src_metadata = tokio::fs::metadata(&src_path).await.unwrap();
        let link_metadata = tokio::fs::metadata(&link_path).await.unwrap();

        use std::os::unix::fs::MetadataExt;
        assert_eq!(src_metadata.ino(), link_metadata.ino());
    }
}
