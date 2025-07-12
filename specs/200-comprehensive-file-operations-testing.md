# Spec 200: Comprehensive File Operations Testing

## Feature Summary

Enhance the test coverage for the core file operations modules (file, copy, stat, template) to meet the comprehensive testing requirements outlined in spec 140. This involves adding integration tests, cross-platform validation tests, property-based tests, and end-to-end workflow tests to ensure robust and reliable file operations across all supported platforms.

**Architecture Note**: This specification focuses on testing infrastructure and does not modify the core file operations modules themselves, ensuring comprehensive validation of existing functionality.

## Goals & Requirements

### Functional Requirements
- **Integration tests**: Full module integration testing with real file system operations
- **Cross-platform tests**: Platform-specific behavior validation (Unix, Windows, macOS)
- **Workflow tests**: End-to-end file operation chains (create → copy → template → stat)
- **Property-based tests**: Fuzz testing for edge cases and invariants
- **Performance tests**: Benchmarking for large file operations
- **Concurrent access tests**: Multi-threaded safety validation

### Non-Functional Requirements
- Test isolation with proper cleanup
- Deterministic test execution across platforms
- Comprehensive error condition coverage
- Performance regression detection
- Memory safety validation
- Test execution time under 30 seconds for full suite

### Success Criteria
- 100% line coverage for file operations modules
- All tests pass on Linux, macOS, and Windows
- Property-based tests validate core invariants
- Integration tests cover real-world usage scenarios
- Performance benchmarks establish baseline metrics
- Zero flaky tests in CI/CD pipeline

## API/Interface Design

### Test Module Structure
```rust
// tests/modules/files/
pub mod integration {
    pub mod file_tests;
    pub mod copy_tests;
    pub mod stat_tests;
    pub mod template_tests;
    pub mod workflow_tests;
}

pub mod property {
    pub mod file_properties;
    pub mod copy_properties;
    pub mod checksum_properties;
}

pub mod platform {
    pub mod unix_tests;
    pub mod windows_tests;
    pub mod cross_platform_tests;
}

pub mod performance {
    pub mod file_benchmarks;
    pub mod copy_benchmarks;
}
```

### Test Helper Interfaces
```rust
pub struct TestEnvironment {
    pub temp_dir: TempDir,
    pub registry: ModuleRegistry,
    pub context: ExecutionContext,
}

impl TestEnvironment {
    pub fn new() -> Self;
    pub fn with_platform_config(platform: &str) -> Self;
    pub async fn execute_module(&self, name: &str, args: ModuleArgs) -> Result<ModuleResult>;
    pub fn create_test_file(&self, path: &str, content: &str) -> PathBuf;
    pub fn create_test_directory(&self, path: &str) -> PathBuf;
}

pub struct FileTestBuilder {
    path: Option<String>,
    state: Option<FileState>,
    mode: Option<String>,
    owner: Option<String>,
    group: Option<String>,
}

impl FileTestBuilder {
    pub fn new() -> Self;
    pub fn path(mut self, path: &str) -> Self;
    pub fn state(mut self, state: FileState) -> Self;
    pub fn mode(mut self, mode: &str) -> Self;
    pub fn build(self) -> ModuleArgs;
}
```

## File and Package Structure

### Integration Test Organization
```
tests/
├── modules/
│   └── files/
│       ├── mod.rs                    # Module declarations and shared utilities
│       ├── integration/
│       │   ├── mod.rs               # Integration test module
│       │   ├── file_tests.rs        # File module integration tests
│       │   ├── copy_tests.rs        # Copy module integration tests
│       │   ├── stat_tests.rs        # Stat module integration tests
│       │   ├── template_tests.rs    # Template module integration tests
│       │   └── workflow_tests.rs    # End-to-end workflow tests
│       ├── property/
│       │   ├── mod.rs               # Property-based test module
│       │   ├── file_properties.rs   # File operation properties
│       │   ├── copy_properties.rs   # Copy operation properties
│       │   └── checksum_properties.rs # Checksum validation properties
│       ├── platform/
│       │   ├── mod.rs               # Platform-specific test module
│       │   ├── unix_tests.rs        # Unix-specific tests
│       │   ├── windows_tests.rs     # Windows-specific tests
│       │   └── cross_platform_tests.rs # Cross-platform compatibility
│       ├── performance/
│       │   ├── mod.rs               # Performance test module
│       │   ├── file_benchmarks.rs   # File operation benchmarks
│       │   └── copy_benchmarks.rs   # Copy operation benchmarks
│       └── helpers/
│           ├── mod.rs               # Test helper module
│           ├── environment.rs       # Test environment setup
│           ├── builders.rs          # Test data builders
│           ├── assertions.rs        # Custom test assertions
│           └── fixtures.rs          # Test fixture management
└── fixtures/
    └── files/
        ├── templates/               # Template test files
        │   ├── simple.txt.j2
        │   ├── config.yaml.j2
        │   └── complex.conf.j2
        ├── test_files/              # Sample files for testing
        │   ├── small.txt
        │   ├── medium.bin
        │   └── large.dat
        └── expected/                # Expected output files
            ├── rendered_simple.txt
            ├── rendered_config.yaml
            └── rendered_complex.conf
```

## Implementation Details

### 1. Integration Tests
```rust
// tests/modules/files/integration/file_tests.rs
#[tokio::test]
async fn test_file_create_with_permissions() {
    let env = TestEnvironment::new();
    
    let args = FileTestBuilder::new()
        .path(&env.temp_path("test_file.txt"))
        .state(FileState::Present)
        .mode("0644")
        .build();
    
    let result = env.execute_module("file", args).await.unwrap();
    
    assert!(result.changed);
    assert!(!result.failed);
    
    // Verify file exists and has correct permissions
    let metadata = std::fs::metadata(env.temp_path("test_file.txt")).unwrap();
    assert!(metadata.is_file());
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(metadata.permissions().mode() & 0o777, 0o644);
    }
}

#[tokio::test]
async fn test_file_directory_creation_recursive() {
    let env = TestEnvironment::new();
    
    let deep_path = env.temp_path("a/b/c/d");
    let args = FileTestBuilder::new()
        .path(&deep_path)
        .state(FileState::Directory)
        .mode("0755")
        .build();
    
    let result = env.execute_module("file", args).await.unwrap();
    
    assert!(result.changed);
    assert!(Path::new(&deep_path).is_dir());
    
    // Verify all parent directories were created
    assert!(Path::new(&env.temp_path("a")).is_dir());
    assert!(Path::new(&env.temp_path("a/b")).is_dir());
    assert!(Path::new(&env.temp_path("a/b/c")).is_dir());
}
```

### 2. Property-Based Tests
```rust
// tests/modules/files/property/checksum_properties.rs
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_checksum_deterministic(content in ".*") {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let env = TestEnvironment::new();
            let file_path = env.create_test_file("test.txt", &content);
            
            // Calculate checksum multiple times
            let checksum1 = calculate_file_checksum(&file_path, ChecksumAlgorithm::Sha256).await.unwrap();
            let checksum2 = calculate_file_checksum(&file_path, ChecksumAlgorithm::Sha256).await.unwrap();
            
            prop_assert_eq!(checksum1, checksum2);
        });
    }
    
    #[test]
    fn test_copy_preserves_content(content in prop::collection::vec(any::<u8>(), 0..1000)) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let env = TestEnvironment::new();
            let src_path = env.temp_path("source.bin");
            let dest_path = env.temp_path("dest.bin");
            
            // Write test content
            tokio::fs::write(&src_path, &content).await.unwrap();
            
            let args = json!({
                "src": src_path,
                "dest": dest_path
            });
            
            let result = env.execute_module("copy", ModuleArgs::from_json(args)).await.unwrap();
            prop_assert!(result.changed || content.is_empty());
            
            // Verify content is identical
            let copied_content = tokio::fs::read(&dest_path).await.unwrap();
            prop_assert_eq!(content, copied_content);
        });
    }
}
```

### 3. Cross-Platform Tests
```rust
// tests/modules/files/platform/cross_platform_tests.rs
#[cfg(unix)]
mod unix_specific {
    use super::*;
    
    #[tokio::test]
    async fn test_unix_permissions() {
        let env = TestEnvironment::new();
        
        let file_path = env.temp_path("unix_perms.txt");
        let args = FileTestBuilder::new()
            .path(&file_path)
            .state(FileState::Present)
            .mode("0755")
            .build();
        
        let result = env.execute_module("file", args).await.unwrap();
        
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(&file_path).unwrap();
        assert_eq!(metadata.permissions().mode() & 0o777, 0o755);
    }
    
    #[tokio::test]
    async fn test_symlink_creation() {
        let env = TestEnvironment::new();
        
        let target = env.create_test_file("target.txt", "content");
        let link_path = env.temp_path("link.txt");
        
        let args = json!({
            "path": link_path,
            "src": target,
            "state": "link"
        });
        
        let result = env.execute_module("file", ModuleArgs::from_json(args)).await.unwrap();
        assert!(result.changed);
        
        let metadata = std::fs::symlink_metadata(&link_path).unwrap();
        assert!(metadata.file_type().is_symlink());
    }
}

#[cfg(windows)]
mod windows_specific {
    use super::*;
    
    #[tokio::test]
    async fn test_windows_attributes() {
        let env = TestEnvironment::new();
        
        // Test Windows-specific file attribute handling
        let file_path = env.temp_path("windows_file.txt");
        let args = FileTestBuilder::new()
            .path(&file_path)
            .state(FileState::Present)
            .build();
        
        let result = env.execute_module("file", args).await.unwrap();
        assert!(result.changed);
        assert!(Path::new(&file_path).exists());
    }
}
```

### 4. Workflow Integration Tests
```rust
// tests/modules/files/integration/workflow_tests.rs
#[tokio::test]
async fn test_complete_file_workflow() {
    let env = TestEnvironment::new();
    
    // Step 1: Create directory structure
    let config_dir = env.temp_path("app/config");
    let args = FileTestBuilder::new()
        .path(&config_dir)
        .state(FileState::Directory)
        .mode("0755")
        .build();
    
    let result = env.execute_module("file", args).await.unwrap();
    assert!(result.changed);
    
    // Step 2: Copy template file
    let template_src = env.fixture_path("templates/app.conf.j2");
    let template_dest = env.temp_path("app/config/app.conf.j2");
    
    let copy_args = json!({
        "src": template_src,
        "dest": template_dest,
        "mode": "0644"
    });
    
    let result = env.execute_module("copy", ModuleArgs::from_json(copy_args)).await.unwrap();
    assert!(result.changed);
    
    // Step 3: Process template
    let config_file = env.temp_path("app/config/app.conf");
    let template_args = json!({
        "src": template_dest,
        "dest": config_file,
        "variables": {
            "app_name": "test_app",
            "port": 8080,
            "debug": true
        }
    });
    
    let result = env.execute_module("template", ModuleArgs::from_json(template_args)).await.unwrap();
    assert!(result.changed);
    
    // Step 4: Verify with stat
    let stat_args = json!({
        "path": config_file,
        "get_checksum": true
    });
    
    let result = env.execute_module("stat", ModuleArgs::from_json(stat_args)).await.unwrap();
    assert!(!result.failed);
    
    let stat_result: StatResult = serde_json::from_value(result.ansible_facts.unwrap()).unwrap();
    assert!(stat_result.exists);
    assert!(stat_result.isreg);
    assert!(stat_result.checksum.is_some());
    
    // Verify rendered content contains expected values
    let content = tokio::fs::read_to_string(&config_file).await.unwrap();
    assert!(content.contains("app_name=test_app"));
    assert!(content.contains("port=8080"));
    assert!(content.contains("debug=true"));
}

#[tokio::test]
async fn test_backup_and_restore_workflow() {
    let env = TestEnvironment::new();
    
    // Create original file
    let original_content = "original content";
    let file_path = env.create_test_file("important.txt", original_content);
    
    // Copy with backup
    let new_content = "updated content";
    let temp_src = env.create_test_file("new.txt", new_content);
    
    let copy_args = json!({
        "src": temp_src,
        "dest": file_path,
        "backup": true
    });
    
    let result = env.execute_module("copy", ModuleArgs::from_json(copy_args)).await.unwrap();
    assert!(result.changed);
    
    // Verify backup was created
    let backup_path = format!("{}.backup", file_path);
    assert!(Path::new(&backup_path).exists());
    
    let backup_content = tokio::fs::read_to_string(&backup_path).await.unwrap();
    assert_eq!(backup_content, original_content);
    
    let current_content = tokio::fs::read_to_string(&file_path).await.unwrap();
    assert_eq!(current_content, new_content);
}
```

### 5. Performance Benchmarks
```rust
// tests/modules/files/performance/copy_benchmarks.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_file_copy(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("copy_small_file_1kb", |b| {
        b.iter(|| {
            rt.block_on(async {
                let env = TestEnvironment::new();
                let content = "x".repeat(1024);
                let src = env.create_test_file("small.txt", &content);
                let dest = env.temp_path("dest_small.txt");
                
                let args = json!({
                    "src": src,
                    "dest": dest
                });
                
                let result = env.execute_module("copy", ModuleArgs::from_json(args)).await.unwrap();
                black_box(result);
            });
        });
    });
    
    c.bench_function("copy_large_file_10mb", |b| {
        b.iter(|| {
            rt.block_on(async {
                let env = TestEnvironment::new();
                let content = "x".repeat(10 * 1024 * 1024);
                let src = env.create_test_file("large.txt", &content);
                let dest = env.temp_path("dest_large.txt");
                
                let args = json!({
                    "src": src,
                    "dest": dest
                });
                
                let result = env.execute_module("copy", ModuleArgs::from_json(args)).await.unwrap();
                black_box(result);
            });
        });
    });
}

criterion_group!(benches, benchmark_file_copy);
criterion_main!(benches);
```

## Testing Strategy

### Unit Test Enhancement
- Expand existing unit tests to cover edge cases
- Add error condition testing for each module
- Validate check mode behavior comprehensively
- Test module parameter validation thoroughly

### Integration Test Categories
1. **Single Module Tests**: Individual module functionality
2. **Cross-Module Tests**: Module interaction scenarios
3. **Platform Tests**: OS-specific behavior validation
4. **Error Recovery Tests**: Failure and recovery scenarios
5. **Performance Tests**: Benchmarking and regression detection

### Property-Based Test Scenarios
- File content preservation across operations
- Checksum consistency and determinism
- Permission preservation and modification
- Atomic operation guarantees
- Backup and restore invariants

### Test Data Management
```rust
// tests/modules/files/helpers/fixtures.rs
pub struct TestFixtures {
    pub templates: HashMap<String, String>,
    pub sample_files: HashMap<String, Vec<u8>>,
    pub expected_outputs: HashMap<String, String>,
}

impl TestFixtures {
    pub fn load() -> Self {
        // Load test fixtures from files/
        let mut fixtures = TestFixtures::default();
        
        // Load templates
        fixtures.templates.insert("simple".to_string(), 
            include_str!("../../../fixtures/files/templates/simple.txt.j2").to_string());
        
        // Load binary test data
        fixtures.sample_files.insert("small".to_string(), 
            include_bytes!("../../../fixtures/files/test_files/small.txt").to_vec());
        
        fixtures
    }
}
```

## Edge Cases & Error Handling

### File System Limitations
- Path length limits testing
- Special character handling in filenames
- Case sensitivity variation testing
- File system permission model differences
- Concurrent access conflict resolution

### Error Scenarios
```rust
#[tokio::test]
async fn test_permission_denied_handling() {
    let env = TestEnvironment::new();
    
    // Create file with restrictive permissions
    let file_path = env.create_test_file("restricted.txt", "content");
    std::fs::set_permissions(&file_path, std::fs::Permissions::from_mode(0o000)).unwrap();
    
    let args = json!({
        "src": file_path,
        "dest": env.temp_path("copy_dest.txt")
    });
    
    let result = env.execute_module("copy", ModuleArgs::from_json(args)).await;
    assert!(result.is_err());
    
    match result.unwrap_err() {
        ModuleExecutionError::FileError(FileError::PermissionDenied { .. }) => {
            // Expected error type
        }
        other => panic!("Unexpected error type: {:?}", other),
    }
}

#[tokio::test]
async fn test_disk_space_exhaustion() {
    // Test behavior when disk space is insufficient
    // This would require platform-specific setup or mocking
}

#[tokio::test]
async fn test_template_syntax_error() {
    let env = TestEnvironment::new();
    
    let invalid_template = "{{ unclosed_variable";
    let template_file = env.create_test_file("invalid.j2", invalid_template);
    
    let args = json!({
        "src": template_file,
        "dest": env.temp_path("output.txt"),
        "variables": {}
    });
    
    let result = env.execute_module("template", ModuleArgs::from_json(args)).await;
    assert!(result.is_err());
}
```

### Concurrency Testing
```rust
#[tokio::test]
async fn test_concurrent_file_operations() {
    let env = TestEnvironment::new();
    let source_file = env.create_test_file("source.txt", "shared content");
    
    // Launch multiple concurrent copy operations
    let tasks: Vec<_> = (0..10)
        .map(|i| {
            let env = env.clone();
            let source = source_file.clone();
            tokio::spawn(async move {
                let dest = env.temp_path(&format!("dest_{}.txt", i));
                let args = json!({
                    "src": source,
                    "dest": dest
                });
                
                env.execute_module("copy", ModuleArgs::from_json(args)).await
            })
        })
        .collect();
    
    // Wait for all operations to complete
    let results = futures::future::join_all(tasks).await;
    
    // Verify all operations succeeded
    for result in results {
        let module_result = result.unwrap().unwrap();
        assert!(!module_result.failed);
    }
}
```

## Dependencies

### Testing Libraries
- **tokio-test = "0.4"** - Async test utilities
- **proptest = "1.4"** - Property-based testing framework
- **criterion = "0.5"** - Benchmarking framework
- **tempfile = "3"** (already available) - Temporary file handling
- **futures = "0.3"** (already available) - Async utilities

### Internal Dependencies
- `crate::modules::files` - File operations modules under test
- `crate::modules::interface` - Module interface traits
- `crate::modules::registry` - Module registry for integration testing

### Platform Dependencies
- **Unix**: No additional dependencies
- **Windows**: Existing winapi dependencies sufficient
- **Cross-platform**: std::fs and tokio::fs abstractions

## Configuration

### Test Configuration
```rust
pub struct TestConfig {
    pub timeout_seconds: u64,          // Default: 30
    pub temp_dir_prefix: String,       // Default: "rustle_test_"
    pub preserve_temp_files: bool,     // Default: false (for debugging)
    pub max_file_size: u64,            // Default: 100MB
    pub enable_property_tests: bool,   // Default: true
    pub property_test_cases: u32,      // Default: 100
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            temp_dir_prefix: "rustle_test_".to_string(),
            preserve_temp_files: false,
            max_file_size: 100 * 1024 * 1024,
            enable_property_tests: true,
            property_test_cases: 100,
        }
    }
}
```

### Environment Variables for Testing
- `RUSTLE_TEST_PRESERVE_TEMP` - Keep temporary files for debugging
- `RUSTLE_TEST_TIMEOUT` - Override default test timeout
- `RUSTLE_TEST_SKIP_PROPERTY` - Skip property-based tests
- `RUSTLE_TEST_VERBOSE` - Enable verbose test output

## Documentation

### Test Documentation Requirements
- Each test module must have comprehensive module-level documentation
- Complex test scenarios require inline comments explaining the logic
- Property-based tests must document the invariants being tested
- Performance benchmarks must document expected baseline performance

### Test Naming Conventions
- Integration tests: `test_<module>_<scenario>`
- Property tests: `test_<property>_<invariant>`
- Platform tests: `test_<platform>_<specific_behavior>`
- Performance tests: `benchmark_<operation>_<scale>`

### Example Documentation
```rust
/// Integration tests for the file module operations.
/// 
/// These tests validate real file system interactions and ensure
/// the file module correctly handles various scenarios including:
/// - File creation and deletion
/// - Directory operations
/// - Permission management
/// - Symbolic and hard link creation
/// - Error handling for edge cases
/// 
/// Tests use temporary directories and clean up after execution.
/// Platform-specific behaviors are tested conditionally using cfg attributes.
mod file_integration_tests {
    /// Test file creation with specific permissions.
    /// 
    /// Validates that:
    /// 1. File is created successfully
    /// 2. Permissions are set correctly (Unix only)
    /// 3. Module returns changed=true for new files
    /// 4. Module returns changed=false for existing files with same permissions
    #[tokio::test]
    async fn test_file_create_with_permissions() {
        // Test implementation...
    }
}
```

## Implementation Priority

### Phase 1: Foundation (Week 1)
1. Test environment and helper infrastructure
2. Basic integration tests for each module
3. Test fixture setup and management

### Phase 2: Core Testing (Week 2)
4. Comprehensive integration test scenarios
5. Cross-platform compatibility tests
6. Error handling and edge case tests

### Phase 3: Advanced Testing (Week 3)
7. Property-based test implementation
8. Workflow integration tests
9. Concurrency and stress tests

### Phase 4: Performance & Polish (Week 4)
10. Performance benchmarks and regression tests
11. CI/CD integration and automation
12. Documentation and test maintenance guides

This specification ensures the file operations modules have robust, comprehensive test coverage that validates functionality across platforms and usage scenarios, meeting the high reliability standards required for deployment automation tools.