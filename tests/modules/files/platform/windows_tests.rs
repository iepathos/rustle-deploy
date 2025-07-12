//! Windows-specific tests for file operations

#[cfg(windows)]
use crate::modules::files::{assert_file_exists, FileTestBuilder, TestEnvironment};
#[cfg(windows)]
use rustle_deploy::modules::files::FileState;

#[cfg(windows)]
#[tokio::test]
async fn test_windows_attributes() {
    let env = TestEnvironment::new();

    let file_path = env.temp_path("windows_file.txt");
    let args = FileTestBuilder::new()
        .path(file_path.to_string_lossy())
        .state(FileState::Present)
        .build();

    let result = env.execute_module("file", args).await.unwrap();
    assert!(result.changed);
    assert!(!result.failed);

    assert_file_exists(&file_path);
}
