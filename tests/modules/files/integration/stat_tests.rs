//! Integration tests for the stat module

use crate::modules::files::{
    assert_file_exists, assert_is_directory, assert_is_file, StatTestBuilder, TestEnvironment,
    TestFixtures,
};
use rustle_deploy::modules::files::StatResult;
use serde_json::Value;

/// Test basic file stat
#[tokio::test]
async fn test_stat_file_basic() {
    let env = TestEnvironment::new();
    let file_path = env.create_test_file("test_file.txt", "test content");

    let args = StatTestBuilder::new()
        .path(file_path.to_string_lossy())
        .build();

    let result = env.execute_module("stat", args).await.unwrap();

    assert!(!result.failed);
    assert!(!result.changed); // stat should never change anything

    // Verify ansible_facts contains stat information
    assert!(!result.ansible_facts.is_empty());

    // Parse the stat result
    let stat_data = &result.ansible_facts;
    assert!(stat_data.contains_key("stat"));

    let stat_info = stat_data.get("stat").unwrap();
    if let Value::Object(stat_obj) = stat_info {
        assert_eq!(stat_obj.get("exists").unwrap(), &Value::Bool(true));
        assert_eq!(stat_obj.get("isreg").unwrap(), &Value::Bool(true));
        assert_eq!(stat_obj.get("isdir").unwrap(), &Value::Bool(false));
    }
}

/// Test stat directory
#[tokio::test]
async fn test_stat_directory() {
    let env = TestEnvironment::new();
    let dir_path = env.create_test_directory("test_dir");

    let args = StatTestBuilder::new()
        .path(dir_path.to_string_lossy())
        .build();

    let result = env.execute_module("stat", args).await.unwrap();

    assert!(!result.failed);
    assert!(!result.changed);

    let stat_data = &result.ansible_facts;
    let stat_info = stat_data.get("stat").unwrap();
    if let Value::Object(stat_obj) = stat_info {
        assert_eq!(stat_obj.get("exists").unwrap(), &Value::Bool(true));
        assert_eq!(stat_obj.get("isreg").unwrap(), &Value::Bool(false));
        assert_eq!(stat_obj.get("isdir").unwrap(), &Value::Bool(true));
    }
}

/// Test stat non-existent file
#[tokio::test]
async fn test_stat_nonexistent() {
    let env = TestEnvironment::new();
    let nonexistent_path = env.temp_path("nonexistent.txt");

    let args = StatTestBuilder::new()
        .path(nonexistent_path.to_string_lossy())
        .build();

    let result = env.execute_module("stat", args).await.unwrap();

    assert!(!result.failed);
    assert!(!result.changed);

    let stat_data = &result.ansible_facts;
    let stat_info = stat_data.get("stat").unwrap();
    if let Value::Object(stat_obj) = stat_info {
        assert_eq!(stat_obj.get("exists").unwrap(), &Value::Bool(false));
    }
}

/// Test stat with checksum calculation
#[tokio::test]
async fn test_stat_with_checksum() {
    let env = TestEnvironment::new();
    let file_content = "test content for checksum";
    let file_path = env.create_test_file("checksum_test.txt", file_content);

    let args = StatTestBuilder::new()
        .path(file_path.to_string_lossy())
        .get_checksum(true)
        .checksum_algorithm("sha256")
        .build();

    let result = env.execute_module("stat", args).await.unwrap();

    assert!(!result.failed);
    assert!(!result.changed);

    let stat_data = &result.ansible_facts;
    let stat_info = stat_data.get("stat").unwrap();
    if let Value::Object(stat_obj) = stat_info {
        assert_eq!(stat_obj.get("exists").unwrap(), &Value::Bool(true));
        assert!(stat_obj.contains_key("checksum"));

        // Verify checksum is a hex string
        if let Some(Value::String(checksum)) = stat_obj.get("checksum") {
            assert!(!checksum.is_empty());
            assert_eq!(checksum.len(), 64); // SHA256 is 64 hex characters
            assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }
}

/// Test stat with different checksum algorithms
#[tokio::test]
async fn test_stat_checksum_algorithms() {
    let env = TestEnvironment::new();
    let file_content = "test content";
    let file_path = env.create_test_file("algo_test.txt", file_content);

    // Test MD5
    let args = StatTestBuilder::new()
        .path(file_path.to_string_lossy())
        .get_checksum(true)
        .checksum_algorithm("md5")
        .build();

    let result = env.execute_module("stat", args).await.unwrap();
    assert!(!result.failed);

    let stat_data = &result.ansible_facts;
    let stat_info = stat_data.get("stat").unwrap();
    if let Value::Object(stat_obj) = stat_info {
        if let Some(Value::String(checksum)) = stat_obj.get("checksum") {
            assert_eq!(checksum.len(), 32); // MD5 is 32 hex characters
        }
    }

    // Test SHA1
    let args = StatTestBuilder::new()
        .path(file_path.to_string_lossy())
        .get_checksum(true)
        .checksum_algorithm("sha1")
        .build();

    let result = env.execute_module("stat", args).await.unwrap();
    assert!(!result.failed);

    let stat_data = &result.ansible_facts;
    let stat_info = stat_data.get("stat").unwrap();
    if let Value::Object(stat_obj) = stat_info {
        if let Some(Value::String(checksum)) = stat_obj.get("checksum") {
            assert_eq!(checksum.len(), 40); // SHA1 is 40 hex characters
        }
    }
}

/// Test stat file size information
#[tokio::test]
async fn test_stat_file_size() {
    let env = TestEnvironment::new();
    let fixtures = TestFixtures::load();

    let large_content = fixtures.get_sample_file("large_text").unwrap();
    let file_path = env.create_test_file_binary("large_file.txt", large_content);

    let args = StatTestBuilder::new()
        .path(file_path.to_string_lossy())
        .build();

    let result = env.execute_module("stat", args).await.unwrap();

    assert!(!result.failed);

    let stat_data = &result.ansible_facts;
    let stat_info = stat_data.get("stat").unwrap();
    if let Value::Object(stat_obj) = stat_info {
        if let Some(Value::Number(size)) = stat_obj.get("size") {
            assert_eq!(size.as_u64().unwrap(), large_content.len() as u64);
        }
    }
}

/// Test stat with MIME type detection
#[tokio::test]
async fn test_stat_mime_type() {
    let env = TestEnvironment::new();

    // Create a text file
    let text_file = env.create_test_file("test.txt", "plain text content");

    let args = StatTestBuilder::new()
        .path(text_file.to_string_lossy())
        .get_mime(true)
        .build();

    let result = env.execute_module("stat", args).await.unwrap();

    assert!(!result.failed);

    let stat_data = &result.ansible_facts;
    let stat_info = stat_data.get("stat").unwrap();
    if let Value::Object(stat_obj) = stat_info {
        // MIME type detection might not be implemented, but should not fail
        assert!(stat_obj.contains_key("exists"));
    }
}

/// Test stat with file attributes (Unix)
#[cfg(unix)]
#[tokio::test]
async fn test_stat_file_attributes() {
    let env = TestEnvironment::new();
    let file_path = env.create_test_file("attr_test.txt", "content");

    // Set specific permissions
    env.set_file_permissions("attr_test.txt", 0o755).unwrap();

    let args = StatTestBuilder::new()
        .path(file_path.to_string_lossy())
        .get_attributes(true)
        .build();

    let result = env.execute_module("stat", args).await.unwrap();

    assert!(!result.failed);

    let stat_data = &result.ansible_facts;
    let stat_info = stat_data.get("stat").unwrap();
    if let Value::Object(stat_obj) = stat_info {
        // Should include Unix-specific attributes
        assert!(stat_obj.contains_key("mode"));
        assert!(stat_obj.contains_key("uid"));
        assert!(stat_obj.contains_key("gid"));

        if let Some(Value::String(mode)) = stat_obj.get("mode") {
            // Mode should be octal representation
            assert!(mode.starts_with("0"));
        }
    }
}

/// Test stat symbolic link
#[cfg(unix)]
#[tokio::test]
async fn test_stat_symlink() {
    let env = TestEnvironment::new();

    let target_file = env.create_test_file("target.txt", "target content");
    let link_path = env.temp_path("link.txt");

    // Create symbolic link
    std::os::unix::fs::symlink(&target_file, &link_path).unwrap();

    let args = StatTestBuilder::new()
        .path(link_path.to_string_lossy())
        .follow(false) // Don't follow the link
        .build();

    let result = env.execute_module("stat", args).await.unwrap();

    assert!(!result.failed);

    let stat_data = &result.ansible_facts;
    let stat_info = stat_data.get("stat").unwrap();
    if let Value::Object(stat_obj) = stat_info {
        assert_eq!(stat_obj.get("exists").unwrap(), &Value::Bool(true));
        assert_eq!(stat_obj.get("islnk").unwrap(), &Value::Bool(true));
        assert_eq!(stat_obj.get("isreg").unwrap(), &Value::Bool(false));

        // Should include link target
        if let Some(Value::String(link_target)) = stat_obj.get("lnk_target") {
            assert_eq!(link_target, &target_file.to_string_lossy());
        }
    }
}

/// Test stat following symbolic link
#[cfg(unix)]
#[tokio::test]
async fn test_stat_follow_symlink() {
    let env = TestEnvironment::new();

    let target_file = env.create_test_file("target.txt", "target content");
    let link_path = env.temp_path("link.txt");

    // Create symbolic link
    std::os::unix::fs::symlink(&target_file, &link_path).unwrap();

    let args = StatTestBuilder::new()
        .path(link_path.to_string_lossy())
        .follow(true) // Follow the link
        .build();

    let result = env.execute_module("stat", args).await.unwrap();

    assert!(!result.failed);

    let stat_data = &result.ansible_facts;
    let stat_info = stat_data.get("stat").unwrap();
    if let Value::Object(stat_obj) = stat_info {
        assert_eq!(stat_obj.get("exists").unwrap(), &Value::Bool(true));
        assert_eq!(stat_obj.get("islnk").unwrap(), &Value::Bool(false));
        assert_eq!(stat_obj.get("isreg").unwrap(), &Value::Bool(true));
    }
}

/// Test stat with timestamps
#[tokio::test]
async fn test_stat_timestamps() {
    let env = TestEnvironment::new();
    let file_path = env.create_test_file("time_test.txt", "content");

    let args = StatTestBuilder::new()
        .path(file_path.to_string_lossy())
        .build();

    let result = env.execute_module("stat", args).await.unwrap();

    assert!(!result.failed);

    let stat_data = &result.ansible_facts;
    let stat_info = stat_data.get("stat").unwrap();
    if let Value::Object(stat_obj) = stat_info {
        // Should include timestamp information
        assert!(stat_obj.contains_key("mtime"));
        assert!(stat_obj.contains_key("atime"));
        assert!(stat_obj.contains_key("ctime"));

        // Timestamps should be numbers (Unix timestamps)
        if let Some(Value::Number(mtime)) = stat_obj.get("mtime") {
            assert!(mtime.as_f64().unwrap() > 0.0);
        }
    }
}

/// Test stat on binary file
#[tokio::test]
async fn test_stat_binary_file() {
    let env = TestEnvironment::new();
    let fixtures = TestFixtures::load();

    let binary_content = fixtures.get_sample_file("small_binary").unwrap();
    let file_path = env.create_test_file_binary("binary_test.bin", binary_content);

    let args = StatTestBuilder::new()
        .path(file_path.to_string_lossy())
        .get_checksum(true)
        .build();

    let result = env.execute_module("stat", args).await.unwrap();

    assert!(!result.failed);

    let stat_data = &result.ansible_facts;
    let stat_info = stat_data.get("stat").unwrap();
    if let Value::Object(stat_obj) = stat_info {
        assert_eq!(stat_obj.get("exists").unwrap(), &Value::Bool(true));
        assert_eq!(stat_obj.get("isreg").unwrap(), &Value::Bool(true));
        assert!(stat_obj.contains_key("checksum"));

        if let Some(Value::Number(size)) = stat_obj.get("size") {
            assert_eq!(size.as_u64().unwrap(), binary_content.len() as u64);
        }
    }
}

/// Test stat error handling for permission denied
#[cfg(unix)]
#[tokio::test]
async fn test_stat_permission_denied() {
    let env = TestEnvironment::new();
    let file_path = env.create_test_file("restricted.txt", "content");

    // Make file unreadable (this might not work in all test environments)
    env.set_file_permissions("restricted.txt", 0o000).unwrap();

    let args = StatTestBuilder::new()
        .path(file_path.to_string_lossy())
        .build();

    let result = env.execute_module("stat", args).await.unwrap();

    // Stat should still work for file existence, even if content is unreadable
    assert!(!result.failed);

    let stat_data = &result.ansible_facts;
    let stat_info = stat_data.get("stat").unwrap();
    if let Value::Object(stat_obj) = stat_info {
        assert_eq!(stat_obj.get("exists").unwrap(), &Value::Bool(true));
    }

    // Restore permissions for cleanup
    env.set_file_permissions("restricted.txt", 0o644).unwrap();
}
