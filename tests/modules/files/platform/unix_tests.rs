//! Unix-specific tests for file operations

#[cfg(unix)]
use crate::modules::files::{
    assert_file_permissions, assert_is_symlink, FileTestBuilder, TestEnvironment,
};
#[cfg(unix)]
use rustle_deploy::modules::files::FileState;

#[cfg(unix)]
#[tokio::test]
async fn test_unix_permissions() {
    let env = TestEnvironment::new();

    let file_path = env.temp_path("unix_perms.txt");
    let args = FileTestBuilder::new()
        .path(file_path.to_string_lossy())
        .state(FileState::Present)
        .mode("0755")
        .build();

    let result = env.execute_module("file", args).await.unwrap();
    assert!(result.changed);
    assert!(!result.failed);

    assert_file_permissions(&file_path, 0o755).unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn test_symlink_creation() {
    let env = TestEnvironment::new();

    let target = env.create_test_file("target.txt", "content");
    let link_path = env.temp_path("link.txt");

    let args = FileTestBuilder::new()
        .path(link_path.to_string_lossy())
        .src(target.to_string_lossy())
        .state(FileState::Link)
        .build();

    let result = env.execute_module("file", args).await.unwrap();
    assert!(result.changed);
    assert!(!result.failed);

    assert_is_symlink(&link_path);
}
