//! TAR archive format handler

use crate::modules::archive::{
    formats::detection::ArchiveFormat,
    utils::{
        compression::{CompressionReader, CompressionWriter},
        extraction::{ExtractionOptions, ExtractionResult},
    },
};
use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};
use tar::{Archive, Builder};
use tokio::task;

#[derive(Debug, thiserror::Error)]
pub enum TarError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TAR error: {0}")]
    Tar(String),
    #[error("Path error: {0}")]
    Path(String),
    #[error("Compression error: {0}")]
    Compression(String),
}

impl From<crate::modules::archive::utils::compression::CompressionError> for TarError {
    fn from(err: crate::modules::archive::utils::compression::CompressionError) -> Self {
        TarError::Compression(err.to_string())
    }
}

pub struct TarHandler;

impl TarHandler {
    pub fn new() -> Self {
        Self
    }

    /// Extract a TAR archive
    pub async fn extract(
        &self,
        src: &Path,
        dest: &Path,
        format: &ArchiveFormat,
        options: &ExtractionOptions,
    ) -> Result<ExtractionResult, TarError> {
        let src = src.to_path_buf();
        let dest = dest.to_path_buf();
        let format = format.clone();
        let options = options.clone();

        task::spawn_blocking(move || Self::extract_sync(&src, &dest, &format, &options))
            .await
            .map_err(|e| TarError::Tar(format!("Task join error: {}", e)))?
    }

    fn extract_sync(
        src: &Path,
        dest: &Path,
        format: &ArchiveFormat,
        options: &ExtractionOptions,
    ) -> Result<ExtractionResult, TarError> {
        let file = File::open(src)?;
        let reader = BufReader::new(file);

        let mut archive: Archive<Box<dyn std::io::Read>> = match format {
            ArchiveFormat::Tar => Archive::new(Box::new(reader)),
            ArchiveFormat::TarGz => {
                let decoder = CompressionReader::new_gzip(reader)?;
                Archive::new(Box::new(decoder))
            }
            ArchiveFormat::TarBz2 => {
                let decoder = CompressionReader::new_bzip2(reader)?;
                Archive::new(Box::new(decoder))
            }
            ArchiveFormat::TarXz => {
                let decoder = CompressionReader::new_xz(reader)?;
                Archive::new(Box::new(decoder))
            }
            _ => return Err(TarError::Tar("Unsupported TAR format".to_string())),
        };

        // Set preserve permissions if needed
        archive.set_preserve_permissions(true);
        archive.set_preserve_mtime(true);

        if !dest.exists() {
            std::fs::create_dir_all(dest)?;
        }

        let mut extracted_files = Vec::new();
        let mut total_size = 0u64;

        for entry_result in archive.entries()? {
            let mut entry = entry_result?;
            let path = entry.path()?.to_path_buf();

            // Apply include/exclude filters
            if Self::should_skip_entry(&path, options) {
                continue;
            }

            // Check for path traversal attacks
            if Self::is_unsafe_path(&path) {
                tracing::warn!("Skipping unsafe path: {:?}", path);
                continue;
            }

            let dest_path = dest.join(&path);

            // Ensure parent directory exists
            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Check if we should keep newer files
            if options.keep_newer && dest_path.exists() {
                let dest_mtime = dest_path.metadata()?.modified()?;
                if let Ok(archive_mtime) = entry.header().mtime() {
                    let archive_time =
                        std::time::UNIX_EPOCH + std::time::Duration::from_secs(archive_mtime);
                    if dest_mtime > archive_time {
                        continue;
                    }
                }
            }

            // Extract the entry
            entry.unpack(&dest_path)?;
            total_size += entry.header().size()?;

            // Set custom permissions if specified
            if let Some(mode) = &options.mode {
                Self::set_file_permissions(&dest_path, mode)?;
            }

            // Set custom ownership if specified
            if options.owner.is_some() || options.group.is_some() {
                Self::set_file_ownership(&dest_path, &options.owner, &options.group)?;
            }

            extracted_files.push(path);
        }

        Ok(ExtractionResult {
            extracted_files,
            total_size,
        })
    }

    /// Create a TAR archive
    pub async fn create(
        &self,
        sources: &[PathBuf],
        dest: &Path,
        format: &ArchiveFormat,
        compression_level: Option<u8>,
    ) -> Result<(), TarError> {
        let sources = sources.to_vec();
        let dest = dest.to_path_buf();
        let format = format.clone();

        task::spawn_blocking(move || Self::create_sync(&sources, &dest, &format, compression_level))
            .await
            .map_err(|e| TarError::Tar(format!("Task join error: {}", e)))?
    }

    fn create_sync(
        sources: &[PathBuf],
        dest: &Path,
        format: &ArchiveFormat,
        compression_level: Option<u8>,
    ) -> Result<(), TarError> {
        let file = File::create(dest)?;
        let writer = BufWriter::new(file);

        let mut builder = match format {
            ArchiveFormat::Tar => Builder::new(Box::new(writer) as Box<dyn std::io::Write>),
            ArchiveFormat::TarGz => {
                let encoder = CompressionWriter::new_gzip(writer, compression_level)?;
                Builder::new(Box::new(encoder) as Box<dyn std::io::Write>)
            }
            ArchiveFormat::TarBz2 => {
                let encoder = CompressionWriter::new_bzip2(writer, compression_level)?;
                Builder::new(Box::new(encoder) as Box<dyn std::io::Write>)
            }
            ArchiveFormat::TarXz => {
                let encoder = CompressionWriter::new_xz(writer, compression_level)?;
                Builder::new(Box::new(encoder) as Box<dyn std::io::Write>)
            }
            _ => {
                return Err(TarError::Tar(
                    "Unsupported TAR format for creation".to_string(),
                ))
            }
        };

        // Add each source to the archive
        for source in sources {
            if source.is_file() {
                let file_name = source
                    .file_name()
                    .ok_or_else(|| TarError::Path("Invalid file name".to_string()))?;
                builder.append_path_with_name(source, file_name)?;
            } else if source.is_dir() {
                builder.append_dir_all(".", source)?;
            }
        }

        builder.finish()?;
        Ok(())
    }

    fn should_skip_entry(path: &Path, options: &ExtractionOptions) -> bool {
        let path_str = path.to_string_lossy();

        // Check exclude patterns
        if let Some(exclude_patterns) = &options.exclude {
            for pattern in exclude_patterns {
                if glob_match(pattern, &path_str) {
                    return true;
                }
            }
        }

        // Check include patterns (if specified, only include matching files)
        if let Some(include_patterns) = &options.include {
            for pattern in include_patterns {
                if glob_match(pattern, &path_str) {
                    return false; // Found a match, don't skip
                }
            }
            return true; // No include pattern matched, skip
        }

        false
    }

    fn is_unsafe_path(path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Check for directory traversal
        path_str.contains("../")
            || path_str.starts_with('/')
            || path_str.contains("\\..\\")
            || path_str.starts_with('\\')
    }

    fn set_file_permissions(path: &Path, mode: &str) -> Result<(), TarError> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = u32::from_str_radix(mode, 8)
                .map_err(|e| TarError::Path(format!("Invalid mode: {}", e)))?;
            let permissions = std::fs::Permissions::from_mode(mode);
            std::fs::set_permissions(path, permissions)?;
        }
        #[cfg(not(unix))]
        {
            tracing::warn!("Setting permissions not supported on this platform");
        }
        Ok(())
    }

    fn set_file_ownership(
        path: &Path,
        owner: &Option<String>,
        group: &Option<String>,
    ) -> Result<(), TarError> {
        #[cfg(unix)]
        {
            use nix::unistd::{chown, Gid, Uid};

            let uid = if let Some(owner) = owner {
                Some(owner.parse::<u32>().map(Uid::from_raw).or_else(|_| {
                    // Try to resolve username to UID
                    nix::unistd::User::from_name(owner)
                        .map(|user| user.map(|u| u.uid))
                        .unwrap_or(None)
                        .ok_or_else(|| TarError::Path(format!("Unknown user: {}", owner)))
                })?)
            } else {
                None
            };

            let gid = if let Some(group) = group {
                Some(group.parse::<u32>().map(Gid::from_raw).or_else(|_| {
                    // Try to resolve group name to GID
                    nix::unistd::Group::from_name(group)
                        .map(|group| group.map(|g| g.gid))
                        .unwrap_or(None)
                        .ok_or_else(|| TarError::Path(format!("Unknown group: {}", group)))
                })?)
            } else {
                None
            };

            chown(path, uid, gid)
                .map_err(|e| TarError::Path(format!("Failed to change ownership: {}", e)))?;
        }
        #[cfg(not(unix))]
        {
            tracing::warn!("Setting ownership not supported on this platform");
        }
        Ok(())
    }
}

// Simple glob matching function
fn glob_match(pattern: &str, text: &str) -> bool {
    // This is a simplified glob matcher
    // For production use, consider using the `glob` crate
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            let prefix = parts[0];
            let suffix = parts[1];
            return text.starts_with(prefix) && text.ends_with(suffix);
        }
    }
    pattern == text
}

impl Default for TarHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_unsafe_path_detection() {
        assert!(TarHandler::is_unsafe_path(Path::new("../etc/passwd")));
        assert!(TarHandler::is_unsafe_path(Path::new("/etc/passwd")));
        assert!(!TarHandler::is_unsafe_path(Path::new("safe/path.txt")));
    }

    #[test]
    fn test_glob_matching() {
        assert!(glob_match("*.txt", "file.txt"));
        assert!(glob_match("test*", "test123"));
        assert!(!glob_match("*.txt", "file.log"));
        assert!(glob_match("exact", "exact"));
    }
}
