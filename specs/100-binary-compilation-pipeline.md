# Spec 100: Binary Compilation Pipeline

## Feature Summary

Implement the complete binary compilation pipeline that transforms generated Rust templates into executable binaries using zigbuild for cross-platform compatibility. This bridges the critical gap between the current "analysis-only" state and actual binary compilation and deployment, enabling rustle-deploy to generate self-contained executables with embedded task execution for target platforms.

**Problem it solves**: The current implementation generates Rust source templates but lacks the compilation pipeline to convert them into deployable binaries. Without this, rustle-deploy remains in analysis mode and cannot deliver the core promise of zero-infrastructure binary deployment.

**High-level approach**: Create a comprehensive compilation pipeline that takes generated templates, creates temporary Rust projects, invokes cargo/zigbuild for cross-compilation, and produces optimized binaries ready for deployment to target hosts.

## Goals & Requirements

### Functional Requirements
- Transform `GeneratedTemplate` structs into deployable binary executables
- Create temporary Rust project directories with proper structure
- Invoke cargo zigbuild for cross-platform compilation
- Support compilation for macOS ARM64 as the primary test target
- Handle compilation errors and provide detailed feedback
- Optimize binary size and performance for remote deployment
- Cache compiled binaries to avoid redundant compilation
- Support incremental compilation when source hasn't changed
- Generate binaries with embedded execution plans and modules
- Provide compilation progress tracking and reporting

### Non-functional Requirements
- **Performance**: Compile binaries for single host in <30 seconds
- **Efficiency**: Reuse compilation cache to reduce rebuild time by 90%+
- **Reliability**: 99%+ compilation success rate for supported targets
- **Size**: Generate optimized binaries <20MB for typical execution plans
- **Cross-platform**: Support compilation for all zigbuild-supported targets

### Success Criteria
- Successfully compile binaries from generated templates
- Execute compiled binaries on macOS ARM64 localhost
- Demonstrate 5x+ performance improvement over SSH execution
- Integrate seamlessly with existing template generation system
- Support both debug and release compilation modes
- Enable end-to-end rustle-plan → binary → execution workflow

## API/Interface Design

### Core Compilation Pipeline

```rust
pub struct BinaryCompiler {
    config: CompilerConfig,
    cache: CompilationCache,
    project_manager: ProjectManager,
    process_executor: ProcessExecutor,
}

impl BinaryCompiler {
    pub fn new(config: CompilerConfig) -> Self;
    
    pub async fn compile_binary(
        &self,
        template: &GeneratedTemplate,
        target_spec: &TargetSpecification,
    ) -> Result<CompiledBinary, CompilationError>;
    
    pub async fn compile_for_deployment(
        &self,
        execution_plan: &RustlePlanOutput,
        hosts: &[String],
    ) -> Result<Vec<CompiledBinary>, CompilationError>;
    
    pub fn check_cache(
        &self,
        template_hash: &str,
        target: &str,
    ) -> Option<CompiledBinary>;
    
    pub async fn cleanup_temp_projects(&self) -> Result<(), std::io::Error>;
}

#[derive(Debug, Clone)]
pub struct CompiledBinary {
    pub binary_id: String,
    pub target_triple: String,
    pub binary_path: PathBuf,
    pub binary_data: Vec<u8>,
    pub size: u64,
    pub checksum: String,
    pub compilation_time: Duration,
    pub optimization_level: OptimizationLevel,
    pub template_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct TargetSpecification {
    pub target_triple: String,
    pub optimization_level: OptimizationLevel,
    pub strip_debug: bool,
    pub enable_lto: bool,
    pub target_cpu: Option<String>,
    pub features: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum OptimizationLevel {
    Debug,
    Release,
    ReleaseWithDebugInfo,
    MinimalSize,
}

pub struct ProjectManager {
    temp_dir: PathBuf,
    template_writer: TemplateWriter,
}

impl ProjectManager {
    pub async fn create_rust_project(
        &self,
        template: &GeneratedTemplate,
    ) -> Result<RustProject, ProjectError>;
    
    pub async fn write_template_to_project(
        &self,
        project: &RustProject,
        template: &GeneratedTemplate,
    ) -> Result<(), ProjectError>;
    
    pub async fn cleanup_project(&self, project: &RustProject) -> Result<(), std::io::Error>;
}

#[derive(Debug, Clone)]
pub struct RustProject {
    pub project_id: String,
    pub project_dir: PathBuf,
    pub cargo_toml_path: PathBuf,
    pub main_rs_path: PathBuf,
    pub created_at: DateTime<Utc>,
}

pub struct ProcessExecutor {
    zigbuild_available: bool,
    cargo_path: PathBuf,
}

impl ProcessExecutor {
    pub async fn compile_project(
        &self,
        project: &RustProject,
        target_spec: &TargetSpecification,
    ) -> Result<PathBuf, CompilationError>;
    
    pub async fn execute_cargo_zigbuild(
        &self,
        project_dir: &Path,
        target: &str,
        optimization: &OptimizationLevel,
    ) -> Result<PathBuf, CompilationError>;
    
    pub async fn execute_cargo_build(
        &self,
        project_dir: &Path,
        target: &str,
        optimization: &OptimizationLevel,
    ) -> Result<PathBuf, CompilationError>;
}
```

### Template Writing System

```rust
pub struct TemplateWriter {
    file_writer: FileWriter,
    cargo_generator: CargoTomlGenerator,
}

impl TemplateWriter {
    pub async fn write_main_rs(
        &self,
        project: &RustProject,
        template: &GeneratedTemplate,
    ) -> Result<(), std::io::Error>;
    
    pub async fn write_cargo_toml(
        &self,
        project: &RustProject,
        template: &GeneratedTemplate,
    ) -> Result<(), std::io::Error>;
    
    pub async fn write_module_files(
        &self,
        project: &RustProject,
        modules: &[ModuleTemplate],
    ) -> Result<(), std::io::Error>;
    
    pub async fn write_static_files(
        &self,
        project: &RustProject,
        static_files: &[StaticFileTemplate],
    ) -> Result<(), std::io::Error>;
}

pub struct CargoTomlGenerator;

impl CargoTomlGenerator {
    pub fn generate_cargo_toml(
        &self,
        template: &GeneratedTemplate,
        target_spec: &TargetSpecification,
    ) -> Result<String, GenerationError>;
    
    pub fn generate_dependencies(&self, modules: &[ModuleTemplate]) -> HashMap<String, String>;
    
    pub fn generate_build_profile(&self, optimization: &OptimizationLevel) -> String;
}
```

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum CompilationError {
    #[error("Project creation failed: {reason}")]
    ProjectCreationFailed { reason: String },
    
    #[error("Template writing failed: {file} - {reason}")]
    TemplateWritingFailed { file: String, reason: String },
    
    #[error("Cargo compilation failed for target {target}: {stderr}")]
    CargoCompilationFailed { target: String, stderr: String },
    
    #[error("Zigbuild compilation failed for target {target}: {stderr}")]
    ZigbuildCompilationFailed { target: String, stderr: String },
    
    #[error("Binary not found after compilation: {expected_path}")]
    BinaryNotFound { expected_path: String },
    
    #[error("Compilation timeout exceeded: {timeout_secs}s")]
    CompilationTimeout { timeout_secs: u64 },
    
    #[error("Unsupported target architecture: {target}")]
    UnsupportedTarget { target: String },
    
    #[error("Cache corruption detected: {cache_path}")]
    CacheCorruption { cache_path: String },
    
    #[error("Insufficient disk space: required {required}MB, available {available}MB")]
    InsufficientDiskSpace { required: u64, available: u64 },
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("Failed to create project directory: {path} - {reason}")]
    DirectoryCreationFailed { path: String, reason: String },
    
    #[error("Failed to write file: {file} - {reason}")]
    FileWriteFailed { file: String, reason: String },
    
    #[error("Template validation failed: {reason}")]
    TemplateValidationFailed { reason: String },
    
    #[error("Project cleanup failed: {project_id} - {reason}")]
    CleanupFailed { project_id: String, reason: String },
}
```

## File and Package Structure

```
src/compilation/
├── mod.rs                     # Module exports (update existing)
├── compiler.rs                # Main BinaryCompiler implementation
├── project_manager.rs         # Rust project creation and management
├── process_executor.rs        # Cargo/zigbuild process execution
├── template_writer.rs         # Template-to-filesystem bridge
├── cargo_generator.rs         # Cargo.toml generation
├── binary_cache.rs            # Binary compilation caching
└── target_detection.rs        # Target platform detection

src/deploy/
├── mod.rs                     # Module exports (update existing)
├── binary_deployer.rs         # Binary deployment to hosts (updated)
└── local_executor.rs          # Local binary execution for testing

tests/compilation/
├── compiler_tests.rs          # BinaryCompiler unit tests
├── project_manager_tests.rs   # Project creation tests
├── process_executor_tests.rs  # Cargo/zigbuild execution tests
├── integration_tests.rs       # End-to-end compilation tests
└── fixtures/
    ├── templates/              # Test template fixtures
    ├── expected_projects/      # Expected project structures
    └── binaries/               # Expected binary outputs
```

## Implementation Details

### Phase 1: Project Creation and Template Writing
1. Implement `ProjectManager` for creating temporary Rust projects
2. Create `TemplateWriter` to write `GeneratedTemplate` to filesystem
3. Implement `CargoTomlGenerator` for dynamic Cargo.toml creation
4. Add file system operations with proper error handling

### Phase 2: Compilation Process Execution
1. Implement `ProcessExecutor` for cargo/zigbuild invocation
2. Add process monitoring and timeout handling
3. Create binary artifact collection and validation
4. Implement compilation error parsing and reporting

### Phase 3: Caching and Optimization
1. Extend existing `CompilationCache` for binary storage
2. Add template hash-based cache keys
3. Implement cache invalidation strategies
4. Add binary size optimization techniques

### Phase 4: macOS ARM64 Testing Integration
1. Create local binary execution testing
2. Add platform-specific compilation flags
3. Implement localhost deployment testing
4. Create integration with existing CLI commands

### Key Algorithms

**Project Creation Algorithm**:
```rust
impl ProjectManager {
    pub async fn create_rust_project(
        &self,
        template: &GeneratedTemplate,
    ) -> Result<RustProject, ProjectError> {
        let project_id = format!("rustle-{}", uuid::Uuid::new_v4());
        let project_dir = self.temp_dir.join(&project_id);
        
        // Create project directory structure
        tokio::fs::create_dir_all(&project_dir).await?;
        tokio::fs::create_dir_all(project_dir.join("src")).await?;
        
        let cargo_toml_path = project_dir.join("Cargo.toml");
        let main_rs_path = project_dir.join("src").join("main.rs");
        
        Ok(RustProject {
            project_id,
            project_dir,
            cargo_toml_path,
            main_rs_path,
            created_at: Utc::now(),
        })
    }
}
```

**Binary Compilation Algorithm**:
```rust
impl BinaryCompiler {
    pub async fn compile_binary(
        &self,
        template: &GeneratedTemplate,
        target_spec: &TargetSpecification,
    ) -> Result<CompiledBinary, CompilationError> {
        // Check cache first
        let template_hash = template.calculate_hash();
        if let Some(cached) = self.check_cache(&template_hash, &target_spec.target_triple) {
            return Ok(cached);
        }
        
        // Create temporary Rust project
        let project = self.project_manager.create_rust_project(template).await?;
        
        // Write template to project files
        self.project_manager.write_template_to_project(&project, template).await?;
        
        // Compile the project
        let binary_path = self.process_executor.compile_project(&project, target_spec).await?;
        
        // Read binary data and create CompiledBinary
        let binary_data = tokio::fs::read(&binary_path).await?;
        let checksum = sha256::digest(&binary_data);
        
        let compiled = CompiledBinary {
            binary_id: format!("binary-{}", uuid::Uuid::new_v4()),
            target_triple: target_spec.target_triple.clone(),
            binary_path: binary_path.clone(),
            binary_data,
            size: binary_data.len() as u64,
            checksum,
            compilation_time: compilation_start.elapsed(),
            optimization_level: target_spec.optimization_level.clone(),
            template_hash,
            created_at: Utc::now(),
        };
        
        // Cache the result
        self.cache.store_binary(&compiled).await?;
        
        // Cleanup temporary project
        self.project_manager.cleanup_project(&project).await?;
        
        Ok(compiled)
    }
}
```

**Cargo.toml Generation Algorithm**:
```rust
impl CargoTomlGenerator {
    pub fn generate_cargo_toml(
        &self,
        template: &GeneratedTemplate,
        target_spec: &TargetSpecification,
    ) -> Result<String, GenerationError> {
        let dependencies = self.generate_dependencies(&template.modules);
        let build_profile = self.generate_build_profile(&target_spec.optimization_level);
        
        let cargo_toml = format!(r#"
[package]
name = "rustle-runner"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
tokio = {{ version = "1", features = ["full"] }}
anyhow = "1"
tracing = "0.1"
{}

[profile.release]
{}

[[bin]]
name = "rustle-runner"
path = "src/main.rs"
"#,
            dependencies.iter()
                .map(|(name, version)| format!("{} = \"{}\"", name, version))
                .collect::<Vec<_>>()
                .join("\n"),
            build_profile
        );
        
        Ok(cargo_toml)
    }
}
```

**Process Execution Algorithm**:
```rust
impl ProcessExecutor {
    pub async fn compile_project(
        &self,
        project: &RustProject,
        target_spec: &TargetSpecification,
    ) -> Result<PathBuf, CompilationError> {
        let binary_path = if self.zigbuild_available {
            self.execute_cargo_zigbuild(
                &project.project_dir,
                &target_spec.target_triple,
                &target_spec.optimization_level,
            ).await?
        } else {
            self.execute_cargo_build(
                &project.project_dir,
                &target_spec.target_triple,
                &target_spec.optimization_level,
            ).await?
        };
        
        // Verify binary exists and is executable
        if !binary_path.exists() {
            return Err(CompilationError::BinaryNotFound {
                expected_path: binary_path.display().to_string(),
            });
        }
        
        Ok(binary_path)
    }
    
    pub async fn execute_cargo_zigbuild(
        &self,
        project_dir: &Path,
        target: &str,
        optimization: &OptimizationLevel,
    ) -> Result<PathBuf, CompilationError> {
        let mut cmd = tokio::process::Command::new("cargo");
        
        cmd.arg("zigbuild")
           .arg("--target")
           .arg(target)
           .current_dir(project_dir);
        
        match optimization {
            OptimizationLevel::Release => {
                cmd.arg("--release");
            }
            OptimizationLevel::MinimalSize => {
                cmd.arg("--release");
                cmd.env("RUSTFLAGS", "-C opt-level=z -C lto=fat -C strip=symbols");
            }
            OptimizationLevel::Debug => {
                // Default debug build
            }
            OptimizationLevel::ReleaseWithDebugInfo => {
                cmd.arg("--release");
                cmd.env("RUSTFLAGS", "-C debug-assertions=on");
            }
        }
        
        let output = cmd.output().await.map_err(|e| {
            CompilationError::ZigbuildCompilationFailed {
                target: target.to_string(),
                stderr: e.to_string(),
            }
        })?;
        
        if !output.status.success() {
            return Err(CompilationError::ZigbuildCompilationFailed {
                target: target.to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }
        
        // Determine binary path based on target and optimization
        let profile_dir = match optimization {
            OptimizationLevel::Debug => "debug",
            _ => "release",
        };
        
        let binary_path = project_dir
            .join("target")
            .join(target)
            .join(profile_dir)
            .join("rustle-runner");
        
        Ok(binary_path)
    }
}
```

## Testing Strategy

### Unit Tests
- **ProjectManager Tests**: Project creation, template writing, cleanup
- **ProcessExecutor Tests**: Cargo/zigbuild execution, error handling
- **TemplateWriter Tests**: File writing, Cargo.toml generation
- **BinaryCompiler Tests**: End-to-end compilation workflow

### Integration Tests
- **macOS ARM64 Compilation**: Complete compilation pipeline on localhost
- **Binary Execution**: Test compiled binaries execute correctly
- **Cache Integration**: Test caching reduces compilation time
- **Error Scenarios**: Test compilation failures and recovery

### Test Infrastructure
```
tests/fixtures/compilation/
├── templates/
│   ├── simple_template.json      # Basic execution plan template
│   ├── complex_template.json     # Multi-module template
│   └── minimal_template.json     # Minimal viable template
├── expected_binaries/
│   ├── checksums/                # Expected binary checksums
│   └── metadata/                 # Expected compilation metadata
└── test_projects/
    ├── valid_project/            # Expected project structure
    └── invalid_project/          # Error test cases
```

### macOS ARM64 Testing
```rust
#[cfg(test)]
mod macos_arm64_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_compile_for_localhost_arm64() {
        let template = create_test_template();
        let target_spec = TargetSpecification {
            target_triple: "aarch64-apple-darwin".to_string(),
            optimization_level: OptimizationLevel::Release,
            strip_debug: true,
            enable_lto: true,
            target_cpu: Some("apple-m1".to_string()),
            features: vec![],
        };
        
        let compiler = BinaryCompiler::new(test_config());
        let binary = compiler.compile_binary(&template, &target_spec).await.unwrap();
        
        assert!(binary.binary_path.exists());
        assert!(binary.size > 0);
        assert!(!binary.checksum.is_empty());
        
        // Test binary execution
        let output = std::process::Command::new(&binary.binary_path)
            .output()
            .expect("Failed to execute binary");
        
        assert!(output.status.success());
    }
    
    #[tokio::test]
    async fn test_caching_reduces_compilation_time() {
        let template = create_test_template();
        let target_spec = create_test_target_spec();
        let compiler = BinaryCompiler::new(test_config());
        
        // First compilation
        let start = std::time::Instant::now();
        let binary1 = compiler.compile_binary(&template, &target_spec).await.unwrap();
        let first_duration = start.elapsed();
        
        // Second compilation (should hit cache)
        let start = std::time::Instant::now();
        let binary2 = compiler.compile_binary(&template, &target_spec).await.unwrap();
        let second_duration = start.elapsed();
        
        assert_eq!(binary1.checksum, binary2.checksum);
        assert!(second_duration < first_duration / 10); // Should be >10x faster
    }
}
```

## Edge Cases & Error Handling

### Compilation Edge Cases
- Target architecture not supported by zigbuild
- Compilation timeout due to large templates
- Insufficient disk space for compilation
- Corrupted template data causing compilation errors
- Missing dependencies in generated Cargo.toml

### File System Edge Cases
- Temporary directory creation failures
- Permission denied when writing project files
- Project cleanup failures leaving stale directories
- Concurrent compilation conflicts in temp directories

### Binary Edge Cases
- Binary size exceeding deployment limits
- Binary execution failures on target platform
- Checksum mismatches indicating corruption
- Cache corruption requiring rebuild

### Recovery Strategies
- Automatic retry with clean project directory on compilation failure
- Fallback to cargo build if zigbuild fails
- Cache invalidation and rebuild on corruption detection
- Graceful degradation to SSH execution if compilation fails
- Detailed error reporting for debugging compilation issues

## Dependencies

### External Crates
```toml
[dependencies]
# Existing dependencies...
uuid = { version = "1", features = ["v4"] }
sha2 = "0.10"                   # Binary checksums
tempfile = "3"                  # Temporary directory management
which = "4"                     # Executable path detection
regex = "1"                     # Output parsing
sysinfo = "0.30"               # System resource monitoring

[dev-dependencies]
test-case = "3"                 # Parameterized tests
assert_cmd = "2"               # Command execution testing
predicates = "3"               # Test assertions
```

### Internal Dependencies
- `rustle_deploy::template` - Template generation system
- `rustle_deploy::compilation::cache` - Compilation caching
- `rustle_deploy::compilation::capabilities` - Capability detection
- `rustle_deploy::types` - Core type definitions

## Configuration

### Compiler Configuration
```rust
#[derive(Debug, Clone)]
pub struct CompilerConfig {
    pub temp_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub compilation_timeout: Duration,
    pub max_parallel_compilations: usize,
    pub enable_cache: bool,
    pub default_optimization: OptimizationLevel,
    pub zigbuild_fallback: bool,
    pub binary_size_limit: Option<u64>,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            temp_dir: std::env::temp_dir().join("rustle-compilation"),
            cache_dir: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".rustle")
                .join("cache"),
            compilation_timeout: Duration::from_secs(300), // 5 minutes
            max_parallel_compilations: num_cpus::get(),
            enable_cache: true,
            default_optimization: OptimizationLevel::Release,
            zigbuild_fallback: true,
            binary_size_limit: Some(50 * 1024 * 1024), // 50MB
        }
    }
}
```

### Environment Variables
- `RUSTLE_COMPILE_TEMP_DIR`: Override temporary compilation directory
- `RUSTLE_COMPILE_TIMEOUT`: Override compilation timeout seconds
- `RUSTLE_COMPILE_CACHE`: Enable/disable compilation caching
- `RUSTLE_BINARY_SIZE_LIMIT`: Maximum binary size in bytes
- `RUSTLE_ZIGBUILD_PATH`: Custom cargo-zigbuild executable path

## Documentation

### Usage Examples
```rust
// Basic binary compilation
let template = generate_template_from_plan(&execution_plan)?;
let target_spec = TargetSpecification {
    target_triple: "aarch64-apple-darwin".to_string(),
    optimization_level: OptimizationLevel::Release,
    strip_debug: true,
    enable_lto: true,
    target_cpu: None,
    features: vec![],
};

let compiler = BinaryCompiler::new(CompilerConfig::default());
let binary = compiler.compile_binary(&template, &target_spec).await?;

println!("Compiled binary: {} ({} bytes)", 
         binary.binary_path.display(), 
         binary.size);

// Test binary execution
let output = std::process::Command::new(&binary.binary_path)
    .output()
    .expect("Failed to execute binary");

if output.status.success() {
    println!("Binary executed successfully");
    println!("Output: {}", String::from_utf8_lossy(&output.stdout));
} else {
    eprintln!("Binary execution failed: {}", 
              String::from_utf8_lossy(&output.stderr));
}
```

### CLI Integration
```bash
# Compile and test locally (new functionality)
rustle-deploy execution_plan.json --compile-only --target aarch64-apple-darwin

# Compile and execute locally for testing
rustle-deploy execution_plan.json --localhost-test

# Show compilation details
rustle-deploy execution_plan.json --compile-only --verbose --dry-run
```

## Integration Points

### Template System Integration
```rust
// Bridge between existing template generation and new compilation
impl From<GeneratedTemplate> for CompilationInput {
    fn from(template: GeneratedTemplate) -> Self {
        CompilationInput {
            main_rs_content: template.main_rs_content,
            cargo_toml_content: template.cargo_toml_content,
            module_files: template.module_files,
            static_files: template.static_files,
            template_hash: template.calculate_hash(),
        }
    }
}
```

### CLI Integration
Update existing CLI commands to use actual compilation:
```rust
// In src/bin/rustle-deploy.rs
async fn run_deployment(execution_plan_path: PathBuf, cli: &RustleDeployCli) -> Result<()> {
    // ... existing analysis code ...
    
    if cli.compile_only {
        let template = generate_binary_template(&execution_plan)?;
        let target_spec = determine_target_spec(&cli, &execution_plan)?;
        
        let compiler = BinaryCompiler::new(CompilerConfig::default());
        let binary = compiler.compile_binary(&template, &target_spec).await?;
        
        println!("✅ Binary compiled successfully:");
        println!("   Path: {}", binary.binary_path.display());
        println!("   Size: {} bytes", binary.size);
        println!("   Target: {}", binary.target_triple);
        
        if cli.localhost_test {
            test_binary_execution(&binary).await?;
        }
    }
    
    // ... rest of implementation
}
```

This specification provides the foundation for transforming rustle-deploy from analysis-only to a fully functional binary compilation and deployment system, starting with macOS ARM64 localhost testing as the primary validation target.