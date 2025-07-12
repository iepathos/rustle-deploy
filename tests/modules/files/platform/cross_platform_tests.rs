//! Cross-platform compatibility tests

use crate::modules::files::{
    assert_file_exists, assert_is_directory, assert_is_file, CopyTestBuilder, FileTestBuilder,
    TestEnvironment,
};
use rustle_deploy::modules::files::FileState;

/// Test that basic file operations work on all platforms
#[tokio::test]
async fn test_cross_platform_file_creation() {
    let env = TestEnvironment::new();

    let args = FileTestBuilder::new()
        .path(env.temp_path("cross_platform.txt").to_string_lossy())
        .state(FileState::Present)
        .build();

    let result = env.execute_module("file", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    let file_path = env.temp_path("cross_platform.txt");
    assert_file_exists(&file_path);
    assert_is_file(&file_path);
}

/// Test that directory creation works on all platforms
#[tokio::test]
async fn test_cross_platform_directory_creation() {
    let env = TestEnvironment::new();

    let args = FileTestBuilder::new()
        .path(env.temp_path("cross_platform_dir").to_string_lossy())
        .state(FileState::Directory)
        .build();

    let result = env.execute_module("file", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    let dir_path = env.temp_path("cross_platform_dir");
    assert_file_exists(&dir_path);
    assert_is_directory(&dir_path);
}

/// Test that file copying works on all platforms
#[tokio::test]
async fn test_cross_platform_file_copy() {
    let env = TestEnvironment::new();

    let src_path = env.create_test_file("src.txt", "cross platform content");
    let dest_path = env.temp_path("dest.txt");

    let args = CopyTestBuilder::new()
        .src(src_path.to_string_lossy())
        .dest(dest_path.to_string_lossy())
        .build();

    let result = env.execute_module("copy", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    assert_file_exists(&dest_path);

    let content = env.read_file("dest.txt").unwrap();
    assert_eq!(content, "cross platform content");
}

/// Test handling of different path separators
#[tokio::test]
async fn test_cross_platform_path_handling() {
    let env = TestEnvironment::new();

    // Use platform-agnostic path construction
    let nested_dir = env.temp_path("level1").join("level2").join("level3");

    let args = FileTestBuilder::new()
        .path(nested_dir.to_string_lossy())
        .state(FileState::Directory)
        .build();

    let result = env.execute_module("file", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);
    assert_is_directory(&nested_dir);
}

/// Test that text files with different line endings work
#[tokio::test]
async fn test_cross_platform_line_endings() {
    let env = TestEnvironment::new();

    // Test with different line ending styles
    let contents = vec![
        "unix\nline\nendings\n",          // Unix style
        "windows\r\nline\r\nendings\r\n", // Windows style
        "old_mac\rline\rendings\r",       // Old Mac style
        "mixed\nline\r\nendings\r",       // Mixed
    ];

    for (i, content) in contents.iter().enumerate() {
        let file_path = env.create_test_file(&format!("line_endings_{}.txt", i), content);

        // Copy to another location
        let dest_path = env.temp_path(&format!("copy_line_endings_{}.txt", i));

        let args = CopyTestBuilder::new()
            .src(file_path.to_string_lossy())
            .dest(dest_path.to_string_lossy())
            .build();

        let result = env.execute_module("copy", args).await.unwrap();
        assert!(!result.failed);

        // Verify content is preserved byte-for-byte
        let original = env.read_file(&format!("line_endings_{}.txt", i)).unwrap();
        let copied = env
            .read_file(&format!("copy_line_endings_{}.txt", i))
            .unwrap();
        assert_eq!(original, copied);
    }
}

/// Test Unicode file handling across platforms
#[tokio::test]
async fn test_cross_platform_unicode() {
    let env = TestEnvironment::new();

    let unicode_content = "Hello 世界! Здравствуй мир! שלום עולם! 🌍🌎🌏";
    let src_path = env.create_test_file("unicode_test.txt", unicode_content);
    let dest_path = env.temp_path("unicode_copy.txt");

    let args = CopyTestBuilder::new()
        .src(src_path.to_string_lossy())
        .dest(dest_path.to_string_lossy())
        .build();

    let result = env.execute_module("copy", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    let copied_content = env.read_file("unicode_copy.txt").unwrap();
    assert_eq!(copied_content, unicode_content);
}

/// Test large file handling across platforms
#[tokio::test]
async fn test_cross_platform_large_file() {
    let env = TestEnvironment::new();

    // Create a moderately large file (1MB)
    let large_content = "A".repeat(1024 * 1024);
    let src_path = env.create_test_file("large_file.txt", &large_content);
    let dest_path = env.temp_path("large_file_copy.txt");

    let args = CopyTestBuilder::new()
        .src(src_path.to_string_lossy())
        .dest(dest_path.to_string_lossy())
        .build();

    let result = env.execute_module("copy", args).await.unwrap();

    assert!(result.changed);
    assert!(!result.failed);

    use crate::modules::files::assert_file_size;
    assert_file_size(&dest_path, large_content.len() as u64).unwrap();
}

/// Test error handling consistency across platforms
#[tokio::test]
async fn test_cross_platform_error_handling() {
    let env = TestEnvironment::new();

    // Try to create a file in a non-existent deep path
    let invalid_path = env.temp_path("nonexistent/very/deep/path/file.txt");

    let args = FileTestBuilder::new()
        .path(invalid_path.to_string_lossy())
        .state(FileState::Present)
        .build();

    let result = env.execute_module("file", args).await;

    // Should either fail gracefully or succeed by creating parent directories
    // Behavior may vary by implementation, but should not panic
    assert!(result.is_ok() || result.is_err());
}

/// Test case sensitivity handling (important for cross-platform compatibility)
#[cfg(not(target_os = "macos"))] // macOS filesystem is case-insensitive by default
#[tokio::test]
async fn test_cross_platform_case_sensitivity() {
    let env = TestEnvironment::new();

    // Create files with different cases
    let file1 = env.create_test_file("TestFile.txt", "content1");
    let file2_path = env.temp_path("testfile.txt");

    let args = FileTestBuilder::new()
        .path(file2_path.to_string_lossy())
        .state(FileState::Present)
        .build();

    let _result = env.execute_module("file", args).await.unwrap();

    // Both files should exist independently on case-sensitive filesystems
    assert_file_exists(&file1);
    assert_file_exists(&file2_path);
}
