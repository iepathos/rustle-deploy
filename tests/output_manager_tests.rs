use chrono::Utc;
use rustle_deploy::compilation::output::BinaryOutputManager;
use rustle_deploy::compilation::{
    BinarySource, CompilationCache, CompiledBinary, OptimizationLevel,
};
use std::path::PathBuf;
use tempfile::TempDir;

fn create_test_cache() -> CompilationCache {
    let temp_dir = TempDir::new().unwrap();
    CompilationCache::new(temp_dir.path().to_path_buf(), true)
}

fn create_test_binary_with_cache() -> (CompiledBinary, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let cache_path = temp_dir.path().join("test-binary");

    // Create a fake binary file in cache
    std::fs::write(&cache_path, b"fake binary data").unwrap();

    let binary = CompiledBinary {
        binary_id: "test-binary-id".to_string(),
        target_triple: "x86_64-unknown-linux-gnu".to_string(),
        binary_path: cache_path.clone(),
        binary_data: b"fake binary data".to_vec(),
        effective_source: BinarySource::Cache { cache_path },
        size: 16,
        checksum: "test-checksum".to_string(),
        compilation_time: std::time::Duration::from_secs(1),
        optimization_level: OptimizationLevel::Release,
        template_hash: "test-template-hash".to_string(),
        created_at: Utc::now(),
    };

    (binary, temp_dir)
}

#[tokio::test]
async fn test_cache_binary_copy() {
    let manager = BinaryOutputManager::new(create_test_cache());
    let (binary, _temp_source) = create_test_binary_with_cache();

    let temp_output = TempDir::new().unwrap();
    let output_path = temp_output.path().join("test-binary");

    let result = manager.copy_to_output(&binary, &output_path).await.unwrap();

    assert!(output_path.exists());
    assert_eq!(result.bytes_copied, binary.size);
    assert!(result.source_verified);
    assert_eq!(result.output_path, output_path);
}

#[tokio::test]
async fn test_memory_binary_copy() {
    let manager = BinaryOutputManager::new(create_test_cache());

    // Create binary with in-memory source
    let binary = CompiledBinary {
        binary_id: "test-binary-id".to_string(),
        target_triple: "x86_64-unknown-linux-gnu".to_string(),
        binary_path: PathBuf::from("/nonexistent/path"),
        binary_data: b"fake binary data".to_vec(),
        effective_source: BinarySource::InMemory,
        size: 16,
        checksum: "test-checksum".to_string(),
        compilation_time: std::time::Duration::from_secs(1),
        optimization_level: OptimizationLevel::Release,
        template_hash: "test-template-hash".to_string(),
        created_at: Utc::now(),
    };

    let temp_output = TempDir::new().unwrap();
    let output_path = temp_output.path().join("test-binary");

    let result = manager.copy_to_output(&binary, &output_path).await.unwrap();

    assert!(output_path.exists());
    assert_eq!(result.bytes_copied, binary.size);
    assert!(result.source_verified);
    assert_eq!(result.output_path, output_path);

    // Verify file contents
    let written_data = std::fs::read(&output_path).unwrap();
    assert_eq!(written_data, b"fake binary data");
}

#[tokio::test]
async fn test_windows_exe_extension() {
    let manager = BinaryOutputManager::new(create_test_cache());
    let (mut binary, _temp_source) = create_test_binary_with_cache();
    binary.target_triple = "x86_64-pc-windows-msvc".to_string();

    let temp_output = TempDir::new().unwrap();
    let output_path = temp_output.path().join("test-binary");

    let result = manager.copy_to_output(&binary, &output_path).await.unwrap();

    assert!(result.output_path.extension() == Some(std::ffi::OsStr::new("exe")));
    assert!(result.output_path.exists());
}
