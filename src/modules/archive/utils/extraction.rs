//! Extraction utilities and common types for archive operations

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ExtractionOptions {
    pub exclude: Option<Vec<String>>,
    pub include: Option<Vec<String>>,
    pub keep_newer: bool,
    pub mode: Option<String>,
    pub owner: Option<String>,
    pub group: Option<String>,
}

impl Default for ExtractionOptions {
    fn default() -> Self {
        Self {
            exclude: None,
            include: None,
            keep_newer: false,
            mode: None,
            owner: None,
            group: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExtractionResult {
    pub extracted_files: Vec<PathBuf>,
    pub total_size: u64,
}

impl ExtractionResult {
    pub fn new() -> Self {
        Self {
            extracted_files: Vec::new(),
            total_size: 0,
        }
    }

    pub fn add_file(&mut self, path: PathBuf, size: u64) {
        self.extracted_files.push(path);
        self.total_size += size;
    }

    pub fn file_count(&self) -> usize {
        self.extracted_files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.extracted_files.is_empty()
    }
}

impl Default for ExtractionResult {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct CreationOptions {
    pub exclude: Option<Vec<String>>,
    pub exclude_path: Option<Vec<String>>,
    pub compression_level: Option<u8>,
    pub remove_source: bool,
    pub mode: Option<String>,
    pub owner: Option<String>,
    pub group: Option<String>,
}

impl Default for CreationOptions {
    fn default() -> Self {
        Self {
            exclude: None,
            exclude_path: None,
            compression_level: None,
            remove_source: false,
            mode: None,
            owner: None,
            group: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreationResult {
    pub created_archive: PathBuf,
    pub archived_files: Vec<PathBuf>,
    pub total_size: u64,
    pub compressed_size: u64,
}

impl CreationResult {
    pub fn new(archive_path: PathBuf) -> Self {
        Self {
            created_archive: archive_path,
            archived_files: Vec::new(),
            total_size: 0,
            compressed_size: 0,
        }
    }

    pub fn add_file(&mut self, path: PathBuf, size: u64) {
        self.archived_files.push(path);
        self.total_size += size;
    }

    pub fn set_compressed_size(&mut self, size: u64) {
        self.compressed_size = size;
    }

    pub fn compression_ratio(&self) -> f64 {
        if self.total_size == 0 {
            0.0
        } else {
            1.0 - (self.compressed_size as f64 / self.total_size as f64)
        }
    }

    pub fn file_count(&self) -> usize {
        self.archived_files.len()
    }
}

/// Utility functions for extraction operations
pub mod utils {
    use std::path::Path;

    /// Check if a path is safe (no directory traversal)
    pub fn is_safe_path(path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Check for directory traversal attempts
        !path_str.contains("../")
            && !path_str.starts_with('/')
            && !path_str.contains("\\..\\")
            && !path_str.starts_with('\\')
    }

    /// Sanitize a path for extraction
    pub fn sanitize_path(path: &Path) -> Option<&Path> {
        if is_safe_path(path) {
            Some(path)
        } else {
            None
        }
    }

    /// Calculate the total size of extracted files
    pub fn calculate_total_size(files: &[std::path::PathBuf]) -> std::io::Result<u64> {
        let mut total_size = 0u64;

        for file in files {
            if file.is_file() {
                total_size += file.metadata()?.len();
            }
        }

        Ok(total_size)
    }

    /// Validate file permissions string
    pub fn validate_permissions(mode: &str) -> bool {
        // Check if it's a valid octal number (3 or 4 digits)
        if mode.len() < 3 || mode.len() > 4 {
            return false;
        }

        mode.chars().all(|c| c.is_ascii_digit() && c <= '7')
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_safe_path_detection() {
            assert!(is_safe_path(Path::new("safe/path.txt")));
            assert!(is_safe_path(Path::new("file.txt")));
            assert!(!is_safe_path(Path::new("../etc/passwd")));
            assert!(!is_safe_path(Path::new("/etc/passwd")));
            assert!(!is_safe_path(Path::new("..\\windows\\system32\\file")));
        }

        #[test]
        fn test_permissions_validation() {
            assert!(validate_permissions("644"));
            assert!(validate_permissions("755"));
            assert!(validate_permissions("0644"));
            assert!(!validate_permissions("888"));
            assert!(!validate_permissions("12"));
            assert!(!validate_permissions("12345"));
            assert!(!validate_permissions("abc"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extraction_result() {
        let mut result = ExtractionResult::new();
        assert!(result.is_empty());
        assert_eq!(result.file_count(), 0);
        assert_eq!(result.total_size, 0);

        result.add_file(PathBuf::from("test.txt"), 100);
        assert!(!result.is_empty());
        assert_eq!(result.file_count(), 1);
        assert_eq!(result.total_size, 100);
    }

    #[test]
    fn test_creation_result() {
        let archive_path = PathBuf::from("test.tar.gz");
        let mut result = CreationResult::new(archive_path.clone());

        assert_eq!(result.created_archive, archive_path);
        assert_eq!(result.file_count(), 0);
        assert_eq!(result.total_size, 0);
        assert_eq!(result.compressed_size, 0);
        assert_eq!(result.compression_ratio(), 0.0);

        result.add_file(PathBuf::from("test.txt"), 1000);
        result.set_compressed_size(500);

        assert_eq!(result.file_count(), 1);
        assert_eq!(result.total_size, 1000);
        assert_eq!(result.compressed_size, 500);
        assert_eq!(result.compression_ratio(), 0.5);
    }
}
