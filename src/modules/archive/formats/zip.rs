//! ZIP archive format handler

use crate::modules::archive::utils::extraction::{ExtractionOptions, ExtractionResult};
use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};
use tokio::task;
use zip::{write::FileOptions, CompressionMethod, ZipArchive, ZipWriter};

#[derive(Debug, thiserror::Error)]
pub enum ZipError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Path error: {0}")]
    Path(String),
}

pub struct ZipHandler;

impl ZipHandler {
    pub fn new() -> Self {
        Self
    }

    /// Extract a ZIP archive
    pub async fn extract(
        &self,
        src: &Path,
        dest: &Path,
        options: &ExtractionOptions,
    ) -> Result<ExtractionResult, ZipError> {
        let src = src.to_path_buf();
        let dest = dest.to_path_buf();
        let options = options.clone();

        task::spawn_blocking(move || Self::extract_sync(&src, &dest, &options))
            .await
            .map_err(|e| ZipError::Path(format!("Task join error: {e}")))?
    }

    fn extract_sync(
        src: &Path,
        dest: &Path,
        options: &ExtractionOptions,
    ) -> Result<ExtractionResult, ZipError> {
        let file = File::open(src)?;
        let reader = BufReader::new(file);
        let mut archive = ZipArchive::new(reader)?;

        if !dest.exists() {
            std::fs::create_dir_all(dest)?;
        }

        let mut extracted_files = Vec::new();
        let mut total_size = 0u64;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let file_path = match file.enclosed_name() {
                Some(path) => path.to_path_buf(),
                None => {
                    tracing::warn!("Skipping file with unsafe path: {}", file.name());
                    continue;
                }
            };

            // Apply include/exclude filters
            if Self::should_skip_entry(&file_path, options) {
                continue;
            }

            let dest_path = dest.join(&file_path);

            // Check if we should keep newer files
            if options.keep_newer && dest_path.exists() {
                let dest_mtime = dest_path.metadata()?.modified()?;
                if let Ok(archive_mtime) = file.last_modified().to_time() {
                    if dest_mtime > archive_mtime {
                        continue;
                    }
                }
            }

            if file.is_dir() {
                std::fs::create_dir_all(&dest_path)?;
            } else {
                // Ensure parent directory exists
                if let Some(parent) = dest_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let mut outfile = File::create(&dest_path)?;
                std::io::copy(&mut file, &mut outfile)?;

                // Set permissions on Unix systems
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Some(mode) = file.unix_mode() {
                        let permissions = std::fs::Permissions::from_mode(mode);
                        std::fs::set_permissions(&dest_path, permissions)?;
                    }
                }

                // Set custom permissions if specified
                if let Some(mode) = &options.mode {
                    Self::set_file_permissions(&dest_path, mode)?;
                }

                // Set custom ownership if specified
                if options.owner.is_some() || options.group.is_some() {
                    Self::set_file_ownership(&dest_path, &options.owner, &options.group)?;
                }
            }

            total_size += file.size();
            extracted_files.push(file_path);
        }

        Ok(ExtractionResult {
            extracted_files,
            total_size,
        })
    }

    /// Create a ZIP archive
    pub async fn create(
        &self,
        sources: &[PathBuf],
        dest: &Path,
        compression_level: Option<u8>,
    ) -> Result<(), ZipError> {
        let sources = sources.to_vec();
        let dest = dest.to_path_buf();

        task::spawn_blocking(move || Self::create_sync(&sources, &dest, compression_level))
            .await
            .map_err(|e| ZipError::Path(format!("Task join error: {e}")))?
    }

    fn create_sync(
        sources: &[PathBuf],
        dest: &Path,
        compression_level: Option<u8>,
    ) -> Result<(), ZipError> {
        let file = File::create(dest)?;
        let writer = BufWriter::new(file);
        let mut zip = ZipWriter::new(writer);

        // Set compression method and level
        let compression_method = CompressionMethod::Deflated;
        let options = FileOptions::default()
            .compression_method(compression_method)
            .compression_level(compression_level.map(|l| l as i32));

        for source in sources {
            if source.is_file() {
                Self::add_file_to_zip(&mut zip, source, &options)?;
            } else if source.is_dir() {
                Self::add_directory_to_zip(&mut zip, source, source, &options)?;
            }
        }

        zip.finish()?;
        Ok(())
    }

    fn add_file_to_zip(
        zip: &mut ZipWriter<BufWriter<File>>,
        file_path: &Path,
        options: &FileOptions,
    ) -> Result<(), ZipError> {
        let name = file_path
            .file_name()
            .ok_or_else(|| ZipError::Path("Invalid file name".to_string()))?
            .to_string_lossy()
            .to_string();

        zip.start_file(name, *options)?;

        let mut file = File::open(file_path)?;
        std::io::copy(&mut file, zip)?;

        Ok(())
    }

    fn add_directory_to_zip(
        zip: &mut ZipWriter<BufWriter<File>>,
        dir_path: &Path,
        base_path: &Path,
        options: &FileOptions,
    ) -> Result<(), ZipError> {
        let walker = walkdir::WalkDir::new(dir_path);

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            let relative_path = path
                .strip_prefix(base_path)
                .map_err(|e| ZipError::Path(format!("Path error: {e}")))?;

            if path.is_file() {
                let name = relative_path.to_string_lossy().to_string();
                zip.start_file(name, *options)?;

                let mut file = File::open(path)?;
                std::io::copy(&mut file, zip)?;
            } else if path.is_dir() && path != base_path {
                let name = format!("{}/", relative_path.to_string_lossy());
                zip.add_directory(name, *options)?;
            }
        }

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

    fn set_file_permissions(path: &Path, mode: &str) -> Result<(), ZipError> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = u32::from_str_radix(mode, 8)
                .map_err(|e| ZipError::Path(format!("Invalid mode: {e}")))?;
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
    ) -> Result<(), ZipError> {
        #[cfg(unix)]
        {
            use nix::unistd::{chown, Gid, Uid};

            let uid = if let Some(owner) = owner {
                Some(owner.parse::<u32>().map(Uid::from_raw).or_else(|_| {
                    nix::unistd::User::from_name(owner)
                        .map(|user| user.map(|u| u.uid))
                        .unwrap_or(None)
                        .ok_or_else(|| ZipError::Path(format!("Unknown user: {owner}")))
                })?)
            } else {
                None
            };

            let gid = if let Some(group) = group {
                Some(group.parse::<u32>().map(Gid::from_raw).or_else(|_| {
                    nix::unistd::Group::from_name(group)
                        .map(|group| group.map(|g| g.gid))
                        .unwrap_or(None)
                        .ok_or_else(|| ZipError::Path(format!("Unknown group: {group}")))
                })?)
            } else {
                None
            };

            chown(path, uid, gid)
                .map_err(|e| ZipError::Path(format!("Failed to change ownership: {e}")))?;
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

impl Default for ZipHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_matching() {
        assert!(glob_match("*.txt", "file.txt"));
        assert!(glob_match("test*", "test123"));
        assert!(!glob_match("*.txt", "file.log"));
        assert!(glob_match("exact", "exact"));
    }
}
