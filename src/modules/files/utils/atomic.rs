//! Atomic file operations for safe file handling

use std::mem::ManuallyDrop;
use std::path::{Path, PathBuf};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use uuid::Uuid;

use super::FileError;

/// Atomic file writer that ensures operations are atomic
pub struct AtomicWriter {
    temp_path: PathBuf,
    final_path: PathBuf,
    temp_file: ManuallyDrop<File>,
}

impl AtomicWriter {
    /// Create a new atomic writer for the target path
    pub async fn new(target_path: impl AsRef<Path>) -> Result<Self, FileError> {
        let final_path = target_path.as_ref().to_path_buf();
        let temp_path = create_temp_file_path(&final_path)?;

        let temp_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)
            .await?;

        Ok(AtomicWriter {
            temp_path,
            final_path,
            temp_file: ManuallyDrop::new(temp_file),
        })
    }

    /// Get a mutable reference to the temporary file for writing
    pub fn file_mut(&mut self) -> &mut File {
        &mut *self.temp_file
    }

    /// Write data to the temporary file
    pub async fn write_all(&mut self, data: &[u8]) -> Result<(), FileError> {
        self.temp_file.write_all(data).await?;
        Ok(())
    }

    /// Flush and commit the atomic operation
    pub async fn commit(mut self) -> Result<(), FileError> {
        self.temp_file.flush().await?;

        // Manually drop the file to ensure it's closed
        unsafe {
            ManuallyDrop::drop(&mut self.temp_file);
        }

        tokio::fs::rename(&self.temp_path, &self.final_path).await?;
        Ok(())
    }

    /// Abort the operation and clean up the temporary file
    pub async fn abort(mut self) -> Result<(), FileError> {
        // Manually drop the file to ensure it's closed
        unsafe {
            ManuallyDrop::drop(&mut self.temp_file);
        }

        let _ = tokio::fs::remove_file(&self.temp_path).await;
        Ok(())
    }
}

impl Drop for AtomicWriter {
    fn drop(&mut self) {
        // Ensure the file is properly closed if it wasn't already
        // Best effort cleanup - ignore errors since we're in a destructor
        let temp_path = self.temp_path.clone();
        tokio::spawn(async move {
            let _ = tokio::fs::remove_file(&temp_path).await;
        });
    }
}

impl AsyncWrite for AtomicWriter {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        std::pin::Pin::new(&mut *self.temp_file).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        std::pin::Pin::new(&mut *self.temp_file).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        std::pin::Pin::new(&mut *self.temp_file).poll_shutdown(cx)
    }
}

/// Generate a unique temporary file path in the same directory as the target
fn create_temp_file_path(target_path: &Path) -> Result<PathBuf, FileError> {
    let parent = target_path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = target_path
        .file_name()
        .ok_or_else(|| FileError::InvalidPermissions {
            mode: "Invalid target path".to_string(),
        })?
        .to_string_lossy();

    let temp_name = format!(".{}.tmp.{}", file_name, Uuid::new_v4().simple());
    Ok(parent.join(temp_name))
}

/// Copy a file atomically from source to destination
pub async fn atomic_copy(src: &Path, dest: &Path) -> Result<(), FileError> {
    let mut writer = AtomicWriter::new(dest).await?;
    let content = tokio::fs::read(src).await?;
    writer.write_all(&content).await?;
    writer.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_atomic_writer_commit() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("test_file.txt");

        let mut writer = AtomicWriter::new(&target_path).await.unwrap();
        writer.write_all(b"Hello, atomic world!").await.unwrap();
        writer.commit().await.unwrap();

        let content = tokio::fs::read_to_string(&target_path).await.unwrap();
        assert_eq!(content, "Hello, atomic world!");
    }

    #[tokio::test]
    async fn test_atomic_writer_abort() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("test_file.txt");

        let mut writer = AtomicWriter::new(&target_path).await.unwrap();
        writer.write_all(b"This should not exist").await.unwrap();
        writer.abort().await.unwrap();

        assert!(!target_path.exists());
    }
}
