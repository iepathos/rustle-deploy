# 120-binary-output-management

## Feature Summary

Fix the binary output management in the compilation pipeline to properly handle cached vs. compiled binaries and ensure reliable binary copying to output directories. Currently, the system fails with "No such file or directory" errors when trying to copy binaries because it attempts to copy from project paths that may not exist when using cached binaries.

The current issue is that the compilation pipeline can return cached binaries (from the cache directory) but the copy operation assumes the binary exists at the project build path. This causes failures in the `--localhost-test` and binary output workflows.

## Goals & Requirements

### Functional Requirements
- **FR1**: Reliably copy compiled/cached binaries to output directory regardless of source location
- **FR2**: Support both fresh compilation and cached binary scenarios seamlessly  
- **FR3**: Provide proper error handling when binary sources are inaccessible
- **FR4**: Maintain backward compatibility with existing CLI options and workflows
- **FR5**: Support all target platforms with proper binary naming (Windows .exe, etc.)

### Non-functional Requirements
- **NFR1**: Operations should complete within 5 seconds for local copies
- **NFR2**: Graceful degradation when filesystem permissions prevent copying
- **NFR3**: Clear error messages indicating the source of copy failures
- **NFR4**: Atomic operations to prevent partial file corruption

### Success Criteria
- Binary copying succeeds for both cached and freshly compiled binaries
- `--localhost-test` mode works reliably without manual cache copying
- Output directory contains properly named binaries for the target platform
- No "No such file or directory" errors during normal operation

## API/Interface Design

### Enhanced CompiledBinary Structure
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledBinary {
    pub binary_id: String,
    pub target_triple: String,
    pub binary_path: PathBuf,           // Original path (project or cache)
    pub binary_data: Vec<u8>,           // Raw binary data
    pub effective_source: BinarySource, // NEW: Track actual source
    pub size: u64,
    pub checksum: String,
    pub compilation_time: Duration,
    pub optimization_level: OptimizationLevel,
    pub template_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinarySource {
    FreshCompilation { project_path: PathBuf },
    Cache { cache_path: PathBuf },
    InMemory, // For future streaming scenarios
}
```

### Binary Output Manager
```rust
pub struct BinaryOutputManager {
    cache: CompilationCache,
    output_strategies: Vec<Box<dyn OutputStrategy>>,
}

pub trait OutputStrategy: Send + Sync {
    async fn copy_binary(
        &self,
        binary: &CompiledBinary,
        output_path: &Path,
    ) -> Result<CopyResult, OutputError>;
    
    fn can_handle(&self, source: &BinarySource) -> bool;
    fn priority(&self) -> u8; // Higher = more preferred
}

pub struct CopyResult {
    pub output_path: PathBuf,
    pub bytes_copied: u64,
    pub copy_duration: Duration,
    pub source_verified: bool,
}
```

### Output Strategies
```rust
// Strategy for copying from cache
pub struct CacheOutputStrategy;

// Strategy for copying from project build directory  
pub struct ProjectOutputStrategy;

// Strategy for copying from in-memory data
pub struct InMemoryOutputStrategy;
```

### Enhanced CLI Integration
```rust
// Enhanced run_compilation function
pub async fn run_compilation(
    execution_plan: &ExecutionPlanSummary,
    cli: &RustleDeployCli,
) -> Result<()> {
    // ... existing compilation logic ...
    
    let binary_manager = BinaryOutputManager::new(compiler.cache());
    let copy_result = binary_manager
        .copy_to_output(&compiled_binary, &output_path)
        .await?;
        
    info!("Binary copied successfully: {} bytes in {:?}", 
          copy_result.bytes_copied, copy_result.copy_duration);
    
    // ... rest of function ...
}
```

## File and Package Structure

```
src/compilation/
├── output/
│   ├── mod.rs                    # Module exports
│   ├── manager.rs                # BinaryOutputManager implementation
│   ├── strategies/
│   │   ├── mod.rs                # Strategy exports
│   │   ├── cache_strategy.rs     # CacheOutputStrategy
│   │   ├── project_strategy.rs   # ProjectOutputStrategy
│   │   └── memory_strategy.rs    # InMemoryOutputStrategy
│   └── error.rs                  # OutputError types
├── compiler.rs                   # Updated to use BinarySource
└── cache.rs                      # Updated to track source info
```

### Module Organization
- `src/compilation/output/` - New output management subsystem
- Enhanced `BinaryCompiler` in `compiler.rs` to track sources
- Updated `CompilationCache` to provide source metadata

## Implementation Details

### Step 1: Enhance CompiledBinary with Source Tracking
```rust
impl BinaryCompiler {
    pub async fn compile_binary(&mut self, template: &GeneratedTemplate, target_spec: &TargetSpecification) -> Result<CompiledBinary, CompilationError> {
        // ... existing logic ...
        
        // Determine the effective source
        let effective_source = if let Some(cached) = self.check_cache(&template_hash, &target_spec.target_triple) {
            BinarySource::Cache { 
                cache_path: self.cache.get_cache_path(&template_hash, &target_spec.target_triple) 
            }
        } else {
            // Fresh compilation
            let project = self.project_manager.create_rust_project(template).await?;
            // ... compilation logic ...
            BinarySource::FreshCompilation { 
                project_path: binary_path.clone() 
            }
        };
        
        let compiled = CompiledBinary {
            // ... existing fields ...
            effective_source,
        };
        
        Ok(compiled)
    }
}
```

### Step 2: Implement Output Strategies
```rust
impl CacheOutputStrategy {
    async fn copy_binary(&self, binary: &CompiledBinary, output_path: &Path) -> Result<CopyResult, OutputError> {
        let start_time = Instant::now();
        
        let cache_path = match &binary.effective_source {
            BinarySource::Cache { cache_path } => cache_path,
            _ => return Err(OutputError::IncompatibleSource),
        };
        
        // Verify cache file exists and is accessible
        if !cache_path.exists() {
            return Err(OutputError::SourceNotFound { 
                path: cache_path.clone() 
            });
        }
        
        // Create output directory if needed
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        // Atomic copy operation
        let temp_path = output_path.with_extension("tmp");
        tokio::fs::copy(cache_path, &temp_path).await?;
        tokio::fs::rename(&temp_path, output_path).await?;
        
        // Verify copy integrity
        let copied_size = tokio::fs::metadata(output_path).await?.len();
        
        Ok(CopyResult {
            output_path: output_path.to_path_buf(),
            bytes_copied: copied_size,
            copy_duration: start_time.elapsed(),
            source_verified: copied_size == binary.size,
        })
    }
}
```

### Step 3: Implement BinaryOutputManager
```rust
impl BinaryOutputManager {
    pub fn new(cache: CompilationCache) -> Self {
        let strategies: Vec<Box<dyn OutputStrategy>> = vec![
            Box::new(CacheOutputStrategy::new()),
            Box::new(ProjectOutputStrategy::new()),
            Box::new(InMemoryOutputStrategy::new()),
        ];
        
        Self { cache, output_strategies: strategies }
    }
    
    pub async fn copy_to_output(&self, binary: &CompiledBinary, output_path: &Path) -> Result<CopyResult, OutputError> {
        // Sort strategies by priority and compatibility
        let mut compatible_strategies: Vec<_> = self.output_strategies
            .iter()
            .filter(|s| s.can_handle(&binary.effective_source))
            .collect();
        compatible_strategies.sort_by_key(|s| std::cmp::Reverse(s.priority()));
        
        let mut last_error = None;
        
        for strategy in compatible_strategies {
            match strategy.copy_binary(binary, output_path).await {
                Ok(result) => {
                    tracing::info!(
                        "Binary copied via {} strategy: {} bytes", 
                        strategy.name(), result.bytes_copied
                    );
                    return Ok(result);
                }
                Err(e) => {
                    tracing::warn!("Strategy {} failed: {}", strategy.name(), e);
                    last_error = Some(e);
                }
            }
        }
        
        Err(last_error.unwrap_or(OutputError::NoCompatibleStrategy))
    }
}
```

### Step 4: Handle Platform-Specific Binary Names
```rust
impl BinaryOutputManager {
    fn adjust_output_path_for_target(&self, base_path: &Path, target_triple: &str) -> PathBuf {
        let mut path = base_path.to_path_buf();
        
        // Add .exe extension for Windows targets
        if target_triple.contains("windows") {
            path.set_extension("exe");
        }
        
        path
    }
}
```

## Testing Strategy

### Unit Tests
```rust
// tests/compilation/output/manager_tests.rs
#[tokio::test]
async fn test_cache_binary_copy() {
    let manager = BinaryOutputManager::new(test_cache());
    let binary = create_test_cached_binary();
    let output_path = temp_dir().join("test-binary");
    
    let result = manager.copy_to_output(&binary, &output_path).await.unwrap();
    
    assert!(output_path.exists());
    assert_eq!(result.bytes_copied, binary.size);
    assert!(result.source_verified);
}

#[tokio::test]
async fn test_strategy_fallback() {
    // Test that manager falls back to alternative strategies
    // when primary strategy fails
}

#[tokio::test]
async fn test_windows_exe_extension() {
    let manager = BinaryOutputManager::new(test_cache());
    let binary = create_test_binary_for_target("x86_64-pc-windows-msvc");
    let output_path = temp_dir().join("test-binary");
    
    let result = manager.copy_to_output(&binary, &output_path).await.unwrap();
    
    assert!(result.output_path.extension() == Some(OsStr::new("exe")));
}
```

### Integration Tests
```rust
// tests/compilation/output/integration_tests.rs
#[tokio::test]
async fn test_end_to_end_binary_output() {
    // Test complete compilation -> cache -> output workflow
}

#[tokio::test] 
async fn test_localhost_test_mode() {
    // Test --localhost-test mode works with both cached and fresh binaries
}
```

## Edge Cases & Error Handling

### Error Scenarios
1. **Cache corruption**: Handle cases where cached binary is corrupted
2. **Permission denied**: Graceful handling of filesystem permission issues
3. **Disk space**: Handle insufficient disk space during copy
4. **Network drives**: Handle slow or unreliable network-mounted outputs
5. **Concurrent access**: Handle multiple processes accessing same cache

### Error Types
```rust
#[derive(Error, Debug)]
pub enum OutputError {
    #[error("Source binary not found: {path}")]
    SourceNotFound { path: PathBuf },
    
    #[error("Incompatible source type for this strategy")]
    IncompatibleSource,
    
    #[error("No compatible output strategy available")]
    NoCompatibleStrategy,
    
    #[error("Copy verification failed: expected {expected} bytes, got {actual}")]
    VerificationFailed { expected: u64, actual: u64 },
    
    #[error("Insufficient disk space: need {needed} bytes, available {available}")]
    InsufficientSpace { needed: u64, available: u64 },
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
```

### Recovery Strategies
- Retry with exponential backoff for transient I/O errors
- Fallback to in-memory copy for permission issues
- Cache invalidation and rebuild for corruption detection
- Temporary file cleanup on failure

## Dependencies

### Internal Dependencies
- `crate::compilation::cache` - For cache path resolution
- `crate::compilation::compiler` - For CompiledBinary integration
- `tokio::fs` - For async file operations

### External Dependencies
- `tracing` - For structured logging of copy operations
- `thiserror` - For error type definitions
- `serde` - For BinarySource serialization

No new external crates required.

## Configuration

### Environment Variables
```bash
RUSTLE_OUTPUT_STRATEGY_PRIORITY="cache,project,memory"  # Strategy preference order
RUSTLE_COPY_RETRY_COUNT=3                               # Number of retry attempts
RUSTLE_COPY_TIMEOUT_SECS=30                            # Copy operation timeout
RUSTLE_VERIFY_COPIES=true                              # Enable copy verification
```

### CLI Options
```rust
#[derive(Parser)]
struct RustleDeployCli {
    // ... existing options ...
    
    /// Skip binary copy verification
    #[arg(long)]
    no_verify_copy: bool,
    
    /// Force specific output strategy
    #[arg(long, value_enum)]
    output_strategy: Option<OutputStrategyType>,
    
    /// Copy retry attempts
    #[arg(long, default_value = "3")]
    copy_retries: u32,
}
```

## Documentation

### API Documentation
- Document all public structs and methods with rustdoc
- Include examples of BinaryOutputManager usage
- Document error conditions and recovery strategies

### README Updates
```markdown
## Binary Output Management

The compilation pipeline now reliably handles binary output regardless of whether
binaries come from cache or fresh compilation:

```bash
# Test compilation with reliable binary output
cargo run --bin rustle-deploy plan.json --localhost-test

# Specify output directory
cargo run --bin rustle-deploy plan.json --output-dir ./binaries

# Force fresh compilation (no cache)
cargo run --bin rustle-deploy plan.json --rebuild
```

### CLI Examples
- `--localhost-test` now works reliably without manual cache copying
- `--output-dir` properly receives binaries from any source
- Cross-platform binary naming (`.exe` on Windows) handled automatically
```

### Error Handling Guide
Document common error scenarios and solutions:
- Cache permission issues → Use `--output-strategy project`
- Network drive timeouts → Increase `--copy-timeout`
- Disk space issues → Clean cache with `--cache-clean`