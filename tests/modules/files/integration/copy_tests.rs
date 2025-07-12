//! Integration tests for the copy module

use crate::modules::files::{
    assert_file_content, assert_file_exists, assert_files_equal, assert_is_file, CopyTestBuilder,
    TestEnvironment, TestFixtures,
};
use std::path::Path;

/// Test basic file copy
#[tokio::test]
async fn test_copy_file_basic() {
    let env = TestEnvironment::new();
    let fixtures = TestFixtures::load();

    let src_content = fixtures.get_sample_file("small_text").unwrap();
    let src_path = env.create_test_file_binary("source.txt", src_content);
    let dest_path = env.temp_path("destination.txt");

    let args = CopyTestBuilder::new()
        .src(src_path.to_string_lossy())
        .dest(dest_path.to_string_lossy())
        .build();

    let result = env.execute_module("copy", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Verify destination file was created and has same content
    assert_file_exists(&dest_path);
    assert_is_file(&dest_path);
    assert_files_equal(&src_path, &dest_path).unwrap();
}

/// Test copy with mode setting
#[tokio::test]
async fn test_copy_with_mode() {
    let env = TestEnvironment::new();

    let src_path = env.create_test_file("source.txt", "test content");
    let dest_path = env.temp_path("destination.txt");

    let args = CopyTestBuilder::new()
        .src(src_path.to_string_lossy())
        .dest(dest_path.to_string_lossy())
        .mode("0644")
        .build();

    let result = env.execute_module("copy", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Verify file was copied with correct permissions
    assert_files_equal(&src_path, &dest_path).unwrap();

    #[cfg(unix)]
    {
        use crate::modules::files::assert_file_permissions;
        assert_file_permissions(&dest_path, 0o644).unwrap();
    }
}

/// Test copy with backup
#[tokio::test]
async fn test_copy_with_backup() {
    let env = TestEnvironment::new();

    let src_path = env.create_test_file("source.txt", "new content");
    let dest_path = env.create_test_file("existing.txt", "original content");

    let args = CopyTestBuilder::new()
        .src(src_path.to_string_lossy())
        .dest(dest_path.to_string_lossy())
        .backup(true)
        .build();

    let result = env.execute_module("copy", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Verify destination has new content
    assert_file_content(&dest_path, "new content").unwrap();

    // Verify backup was created with original content
    let backup_path = format!("{}.backup", dest_path.to_string_lossy());
    assert_file_exists(&backup_path);
    assert_file_content(&backup_path, "original content").unwrap();
}

/// Test copy directory structure
#[tokio::test]
async fn test_copy_directory() {
    let env = TestEnvironment::new();

    // Create source directory structure
    let src_dir = env.create_test_directory("src_dir");
    env.create_test_file("src_dir/file1.txt", "content1");
    env.create_test_file("src_dir/file2.txt", "content2");
    env.create_test_directory("src_dir/subdir");
    env.create_test_file("src_dir/subdir/file3.txt", "content3");

    let dest_dir = env.temp_path("dest_dir");

    let args = CopyTestBuilder::new()
        .src(src_dir.to_string_lossy())
        .dest(dest_dir.to_string_lossy())
        .build();

    let result = env.execute_module("copy", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Verify directory structure was copied
    use crate::modules::files::{assert_directory_contains, assert_is_directory};
    assert_is_directory(&dest_dir);
    assert_directory_contains(&dest_dir, &["file1.txt", "file2.txt", "subdir"]).unwrap();
    assert_is_directory(dest_dir.join("subdir"));
    assert_file_content(dest_dir.join("file1.txt"), "content1").unwrap();
    assert_file_content(dest_dir.join("file2.txt"), "content2").unwrap();
    assert_file_content(dest_dir.join("subdir/file3.txt"), "content3").unwrap();
}

/// Test copy large file
#[tokio::test]
async fn test_copy_large_file() {
    let env = TestEnvironment::new();
    let fixtures = TestFixtures::load();

    let large_content = fixtures.get_sample_file("large_text").unwrap();
    let src_path = env.create_test_file_binary("large_source.txt", large_content);
    let dest_path = env.temp_path("large_destination.txt");

    let args = CopyTestBuilder::new()
        .src(src_path.to_string_lossy())
        .dest(dest_path.to_string_lossy())
        .build();

    let result = env.execute_module("copy", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Verify large file was copied correctly
    assert_files_equal(&src_path, &dest_path).unwrap();

    use crate::modules::files::assert_file_size;
    assert_file_size(&dest_path, large_content.len() as u64).unwrap();
}

/// Test copy binary file
#[tokio::test]
async fn test_copy_binary_file() {
    let env = TestEnvironment::new();
    let fixtures = TestFixtures::load();

    let binary_content = fixtures.get_sample_file("small_binary").unwrap();
    let src_path = env.create_test_file_binary("binary_source.bin", binary_content);
    let dest_path = env.temp_path("binary_destination.bin");

    let args = CopyTestBuilder::new()
        .src(src_path.to_string_lossy())
        .dest(dest_path.to_string_lossy())
        .build();

    let result = env.execute_module("copy", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Verify binary file was copied correctly
    assert_files_equal(&src_path, &dest_path).unwrap();

    use crate::modules::files::assert_file_binary_content;
    assert_file_binary_content(&dest_path, binary_content).unwrap();
}

/// Test copy with force flag
#[tokio::test]
async fn test_copy_force_overwrite() {
    let env = TestEnvironment::new();

    let src_path = env.create_test_file("source.txt", "new content");
    let dest_path = env.create_test_file("existing.txt", "old content");

    // First, try without force (should still work in this implementation)
    let args = CopyTestBuilder::new()
        .src(src_path.to_string_lossy())
        .dest(dest_path.to_string_lossy())
        .force(true)
        .build();

    let result = env.execute_module("copy", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Verify destination was overwritten
    assert_file_content(&dest_path, "new content").unwrap();
}

/// Test copy with preserve attributes
#[tokio::test]
async fn test_copy_preserve_attributes() {
    let env = TestEnvironment::new();

    let src_path = env.create_test_file("source.txt", "content");

    #[cfg(unix)]
    {
        // Set specific permissions on source
        env.set_file_permissions("source.txt", 0o755).unwrap();
    }

    let dest_path = env.temp_path("destination.txt");

    let args = CopyTestBuilder::new()
        .src(src_path.to_string_lossy())
        .dest(dest_path.to_string_lossy())
        .preserve(true)
        .build();

    let result = env.execute_module("copy", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Verify content was copied
    assert_file_content(&dest_path, "content").unwrap();

    #[cfg(unix)]
    {
        // Verify permissions were preserved
        use crate::modules::files::assert_file_permissions;
        assert_file_permissions(&dest_path, 0o755).unwrap();
    }
}

/// Test copy when destination already exists with same content
#[tokio::test]
async fn test_copy_no_change_same_content() {
    let env = TestEnvironment::new();

    let content = "identical content";
    let src_path = env.create_test_file("source.txt", content);
    let dest_path = env.create_test_file("destination.txt", content);

    let args = CopyTestBuilder::new()
        .src(src_path.to_string_lossy())
        .dest(dest_path.to_string_lossy())
        .build();

    let result = env.execute_module("copy", args).await.unwrap();

    // Should not change since content is identical
    assert!(!result.changed);
    assert!(!result.failed);
}

/// Test copy error handling for non-existent source
#[tokio::test]
async fn test_copy_nonexistent_source() {
    let env = TestEnvironment::new();

    let nonexistent_src = env.temp_path("nonexistent.txt");
    let dest_path = env.temp_path("destination.txt");

    let args = CopyTestBuilder::new()
        .src(nonexistent_src.to_string_lossy())
        .dest(dest_path.to_string_lossy())
        .build();

    let result = env.execute_module("copy", args).await;

    // Should fail gracefully
    assert!(result.is_err() || result.unwrap().failed);
}

/// Test copy to directory
#[tokio::test]
async fn test_copy_to_directory() {
    let env = TestEnvironment::new();

    let src_path = env.create_test_file("source.txt", "content");
    let dest_dir = env.create_test_directory("dest_dir");

    let args = CopyTestBuilder::new()
        .src(src_path.to_string_lossy())
        .dest(dest_dir.to_string_lossy())
        .build();

    let result = env.execute_module("copy", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Verify file was copied into directory
    let dest_file = dest_dir.join("source.txt");
    assert_file_exists(&dest_file);
    assert_file_content(&dest_file, "content").unwrap();
}

/// Test copy with symlink handling
#[cfg(unix)]
#[tokio::test]
async fn test_copy_symlink_follow() {
    let env = TestEnvironment::new();

    let target_path = env.create_test_file("target.txt", "target content");
    let link_path = env.temp_path("link.txt");

    // Create symbolic link
    std::os::unix::fs::symlink(&target_path, &link_path).unwrap();

    let dest_path = env.temp_path("destination.txt");

    let args = CopyTestBuilder::new()
        .src(link_path.to_string_lossy())
        .dest(dest_path.to_string_lossy())
        .follow(true)
        .build();

    let result = env.execute_module("copy", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    // Verify target content was copied, not the link
    assert_file_content(&dest_path, "target content").unwrap();
    assert_is_file(&dest_path);
}
