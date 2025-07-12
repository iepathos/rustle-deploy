//! File backup utilities

use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};

use super::FileError;

/// Create a backup of a file with a timestamped suffix
pub async fn create_backup(
    original_path: &Path,
    backup_suffix: Option<&str>,
) -> Result<Option<PathBuf>, FileError> {
    if !original_path.exists() {
        return Ok(None);
    }

    let suffix = backup_suffix.unwrap_or(".backup");
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");

    let backup_path = if suffix.contains('%') {
        // Support strftime-like formatting in suffix
        let formatted_suffix = suffix.replace("%Y%m%d_%H%M%S", &timestamp.to_string());
        original_path.with_extension(format!(
            "{}.{}",
            original_path
                .extension()
                .unwrap_or_default()
                .to_string_lossy(),
            formatted_suffix.trim_start_matches('.')
        ))
    } else {
        // Simple suffix
        PathBuf::from(format!("{}{}", original_path.display(), suffix))
    };

    tokio::fs::copy(original_path, &backup_path).await?;
    Ok(Some(backup_path))
}

/// Create a simple backup with default naming
pub async fn create_simple_backup(original_path: &Path) -> Result<Option<PathBuf>, FileError> {
    create_backup(original_path, Some(".backup")).await
}

/// Restore a file from backup
pub async fn restore_from_backup(
    backup_path: &Path,
    original_path: &Path,
) -> Result<(), FileError> {
    if !backup_path.exists() {
        return Err(FileError::NotFound {
            path: backup_path.display().to_string(),
        });
    }

    tokio::fs::copy(backup_path, original_path).await?;
    Ok(())
}

/// Clean up old backup files
pub async fn cleanup_old_backups(
    directory: &Path,
    pattern: &str,
    keep_count: usize,
) -> Result<Vec<PathBuf>, FileError> {
    let mut backups = Vec::new();
    let mut dir_entries = tokio::fs::read_dir(directory).await?;

    while let Some(entry) = dir_entries.next_entry().await? {
        let path = entry.path();
        if let Some(file_name) = path.file_name() {
            if file_name.to_string_lossy().contains(pattern) {
                if let Ok(metadata) = entry.metadata().await {
                    if let Ok(modified) = metadata.modified() {
                        backups.push((path, modified));
                    }
                }
            }
        }
    }

    // Sort by modification time, newest first
    backups.sort_by(|a, b| b.1.cmp(&a.1));

    let mut removed = Vec::new();
    if backups.len() > keep_count {
        for (path, _) in backups.into_iter().skip(keep_count) {
            if tokio::fs::remove_file(&path).await.is_ok() {
                removed.push(path);
            }
        }
    }

    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_create_backup() {
        let temp_dir = TempDir::new().unwrap();
        let original_path = temp_dir.path().join("test_file.txt");

        // Create original file
        let mut file = tokio::fs::File::create(&original_path).await.unwrap();
        file.write_all(b"original content").await.unwrap();
        file.flush().await.unwrap();

        // Create backup
        let backup_path = create_backup(&original_path, Some(".bak"))
            .await
            .unwrap()
            .unwrap();

        // Verify backup exists and has same content
        let backup_content = tokio::fs::read_to_string(&backup_path).await.unwrap();
        assert_eq!(backup_content, "original content");

        // Verify backup path is correct
        assert!(backup_path.to_string_lossy().ends_with(".bak"));
    }

    #[tokio::test]
    async fn test_restore_from_backup() {
        let temp_dir = TempDir::new().unwrap();
        let original_path = temp_dir.path().join("test_file.txt");
        let backup_path = temp_dir.path().join("test_file.txt.backup");

        // Create backup file
        let mut backup_file = tokio::fs::File::create(&backup_path).await.unwrap();
        backup_file.write_all(b"backup content").await.unwrap();
        backup_file.flush().await.unwrap();

        // Restore from backup
        restore_from_backup(&backup_path, &original_path)
            .await
            .unwrap();

        // Verify restored content
        let restored_content = tokio::fs::read_to_string(&original_path).await.unwrap();
        assert_eq!(restored_content, "backup content");
    }
}
