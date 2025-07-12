//! Integration tests for the file module

use crate::modules::files::{
    assert_file_exists, assert_file_not_exists, assert_is_directory, assert_is_file,
    FileTestBuilder, TestEnvironment,
};
use rustle_deploy::modules::files::FileState;
use std::path::Path;

/// Test file creation with permissions
#[tokio::test]
async fn test_file_create_with_permissions() {
    let env = TestEnvironment::new();

    let args = FileTestBuilder::new()
        .path(env.temp_path("test_file.txt").to_string_lossy())
        .state(FileState::Present)
        .mode("0644")
        .build();

    let result = env.execute_module("file", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Verify file exists and has correct permissions
    let file_path = env.temp_path("test_file.txt");
    assert_file_exists(&file_path);
    assert_is_file(&file_path);

    #[cfg(unix)]
    {
        use crate::modules::files::assert_file_permissions;
        assert_file_permissions(&file_path, 0o644).unwrap();
    }
}

/// Test file creation when file already exists
#[tokio::test]
async fn test_file_create_existing_no_change() {
    let env = TestEnvironment::new();
    let file_path = env.create_test_file("existing.txt", "content");

    let args = FileTestBuilder::new()
        .path(file_path.to_string_lossy())
        .state(FileState::Present)
        .build();

    let result = env.execute_module("file", args).await.unwrap();

    // Should not change since file already exists
    assert!(!result.changed);
    assert!(!result.failed);
}

/// Test directory creation with recursive path
#[tokio::test]
async fn test_directory_create_recursive() {
    let env = TestEnvironment::new();

    let deep_path = env.temp_path("a/b/c/d");
    let args = FileTestBuilder::new()
        .path(deep_path.to_string_lossy())
        .state(FileState::Directory)
        .mode("0755")
        .build();

    let result = env.execute_module("file", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Verify all directories were created
    assert_is_directory(&deep_path);
    assert_is_directory(env.temp_path("a"));
    assert_is_directory(env.temp_path("a/b"));
    assert_is_directory(env.temp_path("a/b/c"));

    #[cfg(unix)]
    {
        use crate::modules::files::assert_file_permissions;
        assert_file_permissions(&deep_path, 0o755).unwrap();
    }
}

/// Test file deletion (absent state)
#[tokio::test]
async fn test_file_delete() {
    let env = TestEnvironment::new();
    let file_path = env.create_test_file("to_delete.txt", "content");

    // Verify file exists first
    assert_file_exists(&file_path);

    let args = FileTestBuilder::new()
        .path(file_path.to_string_lossy())
        .state(FileState::Absent)
        .build();

    let result = env.execute_module("file", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Verify file was deleted
    assert_file_not_exists(&file_path);
}

/// Test directory deletion (absent state)
#[tokio::test]
async fn test_directory_delete() {
    let env = TestEnvironment::new();
    let dir_path = env.create_test_directory("to_delete_dir");

    // Verify directory exists first
    assert_is_directory(&dir_path);

    let args = FileTestBuilder::new()
        .path(dir_path.to_string_lossy())
        .state(FileState::Absent)
        .build();

    let result = env.execute_module("file", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Verify directory was deleted
    assert_file_not_exists(&dir_path);
}

/// Test file touch operation
#[tokio::test]
async fn test_file_touch() {
    let env = TestEnvironment::new();
    let file_path = env.temp_path("touch_test.txt");

    let args = FileTestBuilder::new()
        .path(file_path.to_string_lossy())
        .state(FileState::Touch)
        .build();

    let result = env.execute_module("file", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Verify file was created
    assert_file_exists(&file_path);
    assert_is_file(&file_path);
}

/// Test symbolic link creation (Unix only)
#[cfg(unix)]
#[tokio::test]
async fn test_symlink_creation() {
    let env = TestEnvironment::new();

    let target = env.create_test_file("target.txt", "link target content");
    let link_path = env.temp_path("link.txt");

    let args = FileTestBuilder::new()
        .path(link_path.to_string_lossy())
        .src(target.to_string_lossy())
        .state(FileState::Link)
        .build();

    let result = env.execute_module("file", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Verify symbolic link was created
    use crate::modules::files::assert_is_symlink;
    assert_is_symlink(&link_path);

    // Verify link target is correct
    let link_target = std::fs::read_link(&link_path).unwrap();
    assert_eq!(link_target, target);
}

/// Test hard link creation (Unix only)
#[cfg(unix)]
#[tokio::test]
async fn test_hardlink_creation() {
    let env = TestEnvironment::new();

    let target = env.create_test_file("target.txt", "hard link target content");
    let link_path = env.temp_path("hardlink.txt");

    let args = FileTestBuilder::new()
        .path(link_path.to_string_lossy())
        .src(target.to_string_lossy())
        .state(FileState::Hard)
        .build();

    let result = env.execute_module("file", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Verify hard link was created
    assert_file_exists(&link_path);
    assert_is_file(&link_path);

    // Verify both files have the same content
    use crate::modules::files::assert_files_equal;
    assert_files_equal(&target, &link_path).unwrap();

    // Verify they are the same file (same inode)
    use std::os::unix::fs::MetadataExt;
    let target_metadata = std::fs::metadata(&target).unwrap();
    let link_metadata = std::fs::metadata(&link_path).unwrap();
    assert_eq!(target_metadata.ino(), link_metadata.ino());
}

/// Test permission changes on existing file
#[cfg(unix)]
#[tokio::test]
async fn test_permission_change() {
    let env = TestEnvironment::new();
    let file_path = env.create_test_file("perm_test.txt", "content");

    // Set initial permissions
    env.set_file_permissions("perm_test.txt", 0o600).unwrap();

    let args = FileTestBuilder::new()
        .path(file_path.to_string_lossy())
        .mode("0755")
        .build();

    let result = env.execute_module("file", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Verify permissions were changed
    use crate::modules::files::assert_file_permissions;
    assert_file_permissions(&file_path, 0o755).unwrap();
}

/// Test backup creation when modifying file
#[tokio::test]
async fn test_file_backup() {
    let env = TestEnvironment::new();
    let file_path = env.create_test_file("backup_test.txt", "original content");

    let args = FileTestBuilder::new()
        .path(file_path.to_string_lossy())
        .backup(true)
        .mode("0644")
        .build();

    let result = env.execute_module("file", args).await.unwrap();

    // Should change due to backup flag
    assert!(result.changed);
    assert!(!result.failed);

    // Verify backup was created
    let backup_path = format!("{}.backup", file_path.to_string_lossy());
    assert_file_exists(&backup_path);

    // Verify backup contains original content
    use crate::modules::files::assert_file_content;
    assert_file_content(&backup_path, "original content").unwrap();
}

/// Test error handling for invalid paths
#[tokio::test]
async fn test_invalid_path_error() {
    let env = TestEnvironment::new();

    // Try to create file in non-existent parent directory without recursive flag
    let invalid_path = env.temp_path("nonexistent/deeply/nested/file.txt");

    let args = FileTestBuilder::new()
        .path(invalid_path.to_string_lossy())
        .state(FileState::Present)
        .build();

    let result = env.execute_module("file", args).await;

    // Should fail gracefully
    assert!(result.is_err() || result.unwrap().failed);
}

/// Test check mode (dry run)
#[tokio::test]
async fn test_check_mode() {
    let env = TestEnvironment::new();
    let file_path = env.temp_path("check_mode_test.txt");

    // Create context with check mode enabled
    let mut context = env.context().clone();
    context.check_mode = true;

    let args = FileTestBuilder::new()
        .path(file_path.to_string_lossy())
        .state(FileState::Present)
        .build();

    // For this test, we'd need to modify execute_module to accept custom context
    // This is a simplified version
    let result = env.execute_module("file", args).await.unwrap();

    // In check mode, file should not be created
    // Note: This test would need modification of TestEnvironment to support check mode
    assert_file_not_exists(&file_path);
}
