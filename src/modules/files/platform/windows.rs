//! Windows-specific file operations

use std::path::Path;

use crate::modules::files::utils::FileError;

/// Create a symbolic link (Windows)
pub async fn create_symlink(src: &Path, dest: &Path) -> Result<(), FileError> {
    // Windows requires admin privileges for symlinks in many cases
    // We'll use the std library implementation which handles this
    if src.is_dir() {
        std::os::windows::fs::symlink_dir(src, dest)?;
    } else {
        std::os::windows::fs::symlink_file(src, dest)?;
    }
    Ok(())
}

/// Create a hard link (Windows)
pub async fn create_hardlink(src: &Path, dest: &Path) -> Result<(), FileError> {
    std::fs::hard_link(src, dest)?;
    Ok(())
}

/// Get extended file attributes (Windows)
pub async fn get_extended_attributes(
    _path: &Path,
) -> Result<std::collections::HashMap<String, Vec<u8>>, FileError> {
    // Windows has Alternate Data Streams (ADS) instead of extended attributes
    // For now, return empty map
    Ok(std::collections::HashMap::new())
}

/// Set extended file attributes (Windows)
pub async fn set_extended_attribute(
    _path: &Path,
    _name: &str,
    _value: &[u8],
) -> Result<(), FileError> {
    // This would implement ADS support
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
            supports_symlinks: true,            // NTFS supports symlinks
            supports_hardlinks: true,           // NTFS supports hardlinks
            supports_permissions: false,        // Windows uses ACLs, not Unix permissions
            supports_ownership: false,          // Windows uses different ownership model
            supports_extended_attributes: true, // Windows has ADS
        }
    }
}

/// Windows-specific file attribute management
pub async fn set_file_attributes(path: &Path, attributes: u32) -> Result<(), FileError> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::fileapi::SetFileAttributesW;

    let wide_path: Vec<u16> = OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        if SetFileAttributesW(wide_path.as_ptr(), attributes) == 0 {
            return Err(FileError::PermissionDenied {
                path: path.display().to_string(),
            });
        }
    }

    Ok(())
}

/// Get Windows file attributes
pub async fn get_file_attributes(path: &Path) -> Result<u32, FileError> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::fileapi::{GetFileAttributesW, INVALID_FILE_ATTRIBUTES};

    let wide_path: Vec<u16> = OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let attributes = GetFileAttributesW(wide_path.as_ptr());
        if attributes == INVALID_FILE_ATTRIBUTES {
            return Err(FileError::NotFound {
                path: path.display().to_string(),
            });
        }
        Ok(attributes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

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
    }
}
