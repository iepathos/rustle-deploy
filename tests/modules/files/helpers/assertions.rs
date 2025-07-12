//! Custom assertions for file operations testing

use anyhow::Result;
use std::path::Path;

/// Assert that a file exists
pub fn assert_file_exists<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    assert!(path.exists(), "File does not exist: {}", path.display());
}

/// Assert that a file does not exist
pub fn assert_file_not_exists<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    assert!(!path.exists(), "File should not exist: {}", path.display());
}

/// Assert that a file contains expected content
pub fn assert_file_content<P: AsRef<Path>>(path: P, expected_content: &str) -> Result<()> {
    let path = path.as_ref();
    let actual_content = std::fs::read_to_string(path)?;
    assert_eq!(
        actual_content,
        expected_content,
        "File content mismatch in {}",
        path.display()
    );
    Ok(())
}

/// Assert that a file contains expected binary content
pub fn assert_file_binary_content<P: AsRef<Path>>(path: P, expected_content: &[u8]) -> Result<()> {
    let path = path.as_ref();
    let actual_content = std::fs::read(path)?;
    assert_eq!(
        actual_content,
        expected_content,
        "File binary content mismatch in {}",
        path.display()
    );
    Ok(())
}

/// Assert that a path is a directory
pub fn assert_is_directory<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    assert!(path.is_dir(), "Path is not a directory: {}", path.display());
}

/// Assert that a path is a regular file
pub fn assert_is_file<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    assert!(path.is_file(), "Path is not a file: {}", path.display());
}

/// Assert that a path is a symbolic link
#[cfg(unix)]
pub fn assert_is_symlink<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    let metadata = std::fs::symlink_metadata(path).expect("Failed to get metadata");
    assert!(
        metadata.file_type().is_symlink(),
        "Path is not a symbolic link: {}",
        path.display()
    );
}

/// Assert file permissions (Unix only)
#[cfg(unix)]
pub fn assert_file_permissions<P: AsRef<Path>>(path: P, expected_mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let path = path.as_ref();
    let metadata = std::fs::metadata(path)?;
    let actual_mode = metadata.permissions().mode() & 0o777;

    assert_eq!(
        actual_mode,
        expected_mode,
        "File permissions mismatch for {}: expected {:o}, got {:o}",
        path.display(),
        expected_mode,
        actual_mode
    );
    Ok(())
}

/// Assert file ownership (Unix only)
#[cfg(unix)]
pub fn assert_file_ownership<P: AsRef<Path>>(
    path: P,
    expected_owner: &str,
    expected_group: &str,
) -> Result<()> {
    use std::os::unix::fs::MetadataExt;

    let path = path.as_ref();
    let metadata = std::fs::metadata(path)?;

    // Get owner name from UID
    let owner_name = nix::unistd::User::from_uid(nix::unistd::Uid::from_raw(metadata.uid()))?
        .map(|u| u.name)
        .unwrap_or_else(|| metadata.uid().to_string());

    // Get group name from GID
    let group_name = nix::unistd::Group::from_gid(nix::unistd::Gid::from_raw(metadata.gid()))?
        .map(|g| g.name)
        .unwrap_or_else(|| metadata.gid().to_string());

    assert_eq!(
        owner_name,
        expected_owner,
        "File owner mismatch for {}: expected {}, got {}",
        path.display(),
        expected_owner,
        owner_name
    );

    assert_eq!(
        group_name,
        expected_group,
        "File group mismatch for {}: expected {}, got {}",
        path.display(),
        expected_group,
        group_name
    );

    Ok(())
}

/// Assert that two files have the same content
pub fn assert_files_equal<P1: AsRef<Path>, P2: AsRef<Path>>(path1: P1, path2: P2) -> Result<()> {
    let path1 = path1.as_ref();
    let path2 = path2.as_ref();

    let content1 = std::fs::read(path1)?;
    let content2 = std::fs::read(path2)?;

    assert_eq!(
        content1,
        content2,
        "Files have different content: {} vs {}",
        path1.display(),
        path2.display()
    );

    Ok(())
}

/// Assert that a file size matches expected size
pub fn assert_file_size<P: AsRef<Path>>(path: P, expected_size: u64) -> Result<()> {
    let path = path.as_ref();
    let metadata = std::fs::metadata(path)?;
    let actual_size = metadata.len();

    assert_eq!(
        actual_size,
        expected_size,
        "File size mismatch for {}: expected {} bytes, got {} bytes",
        path.display(),
        expected_size,
        actual_size
    );

    Ok(())
}

/// Assert that a directory contains expected files
pub fn assert_directory_contains<P: AsRef<Path>>(
    dir_path: P,
    expected_files: &[&str],
) -> Result<()> {
    let dir_path = dir_path.as_ref();

    for expected_file in expected_files {
        let file_path = dir_path.join(expected_file);
        assert!(
            file_path.exists(),
            "Directory {} missing expected file: {}",
            dir_path.display(),
            expected_file
        );
    }

    Ok(())
}

/// Assert that a directory is empty
pub fn assert_directory_empty<P: AsRef<Path>>(dir_path: P) -> Result<()> {
    let dir_path = dir_path.as_ref();

    let entries: Vec<_> = std::fs::read_dir(dir_path)?.collect();
    assert!(
        entries.is_empty(),
        "Directory should be empty but contains {} entries: {}",
        entries.len(),
        dir_path.display()
    );

    Ok(())
}

/// Assert that two file timestamps are approximately equal (within 1 second)
pub fn assert_timestamps_approx_equal<P1: AsRef<Path>, P2: AsRef<Path>>(
    path1: P1,
    path2: P2,
) -> Result<()> {
    let path1 = path1.as_ref();
    let path2 = path2.as_ref();

    let metadata1 = std::fs::metadata(path1)?;
    let metadata2 = std::fs::metadata(path2)?;

    let time1 = metadata1.modified()?;
    let time2 = metadata2.modified()?;

    let diff = time1
        .duration_since(time2)
        .unwrap_or_else(|_| time2.duration_since(time1).unwrap());

    assert!(
        diff.as_secs() <= 1,
        "File timestamps differ by more than 1 second: {} vs {}",
        path1.display(),
        path2.display()
    );

    Ok(())
}
