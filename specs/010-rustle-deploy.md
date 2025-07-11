# Spec 010: Rustle Deploy Tool

## Feature Summary

The `rustle-deploy` tool is a specialized binary compiler and deployment manager that takes execution plans from `rustle-plan`, compiles optimized target binaries with embedded execution data, and deploys them to remote hosts. This tool enables the revolutionary performance approach of local execution with minimal network overhead while maintaining the modular unix-style architecture.

**Problem it solves**: Bridges the gap between planning and execution by creating self-contained target binaries that eliminate network round-trips during execution, providing 10x+ performance improvements while preserving the modular tool architecture.

**High-level approach**: Create a standalone binary that reads execution plans with binary deployment specifications, cross-compiles optimized Rust binaries with embedded execution data and modules, and deploys them to target hosts for local execution.

## Goals & Requirements

### Functional Requirements
- Compile target binaries from execution plans with binary deployment specifications
- Cross-compile for different target architectures and operating systems
- Embed execution plans, modules, and static files into target binaries
- Deploy compiled binaries to target hosts via SSH/SCP
- Manage binary lifecycle (deploy, execute, cleanup)
- Support incremental compilation and caching
- Handle compilation failures and fallback to SSH execution
- Provide binary versioning and update mechanisms
- Support custom module compilation and static linking
- Generate deployment reports and metrics

### Non-functional Requirements
- **Performance**: Compile binaries for 100+ hosts in <2 minutes
- **Efficiency**: Reduce network overhead by 80%+ compared to SSH execution
- **Reliability**: 99%+ compilation success rate for supported targets
- **Size**: Keep binary sizes <50MB for typical deployments
- **Security**: Secure binary deployment and execution validation

### Success Criteria
- Binary deployment reduces execution time by 5x+ compared to SSH-only execution
- Compilation cache reduces rebuild time by 90%+ for incremental changes
- Binary size optimization keeps deployment overhead minimal
- Cross-compilation supports all major target platforms (Linux x86_64, ARM64, macOS)
- Integration with modular tools maintains unix-style composability

## API/Interface Design

### Command Line Interface
```bash
rustle-deploy [OPTIONS] [EXECUTION_PLAN]

OPTIONS:
    -i, --inventory <FILE>         Inventory file with target host information
    -o, --output-dir <DIR>         Directory for compiled binaries [default: ./target]
    -t, --target <TRIPLE>          Target architecture (auto-detect from inventory)
    --cache-dir <DIR>              Compilation cache directory
    --incremental                  Enable incremental compilation
    --rebuild                      Force rebuild of all binaries
    --deploy-only                  Deploy existing binaries without compilation
    --compile-only                 Compile binaries without deployment
    --cleanup                      Remove deployed binaries from targets
    --parallel <NUM>               Parallel compilation jobs [default: CPU cores]
    --timeout <SECONDS>            Deployment timeout per host [default: 120]
    --binary-suffix <SUFFIX>       Suffix for binary names
    --strip-symbols                Strip debug symbols from binaries
    --compress                     Compress binaries before deployment
    --verify                       Verify binary integrity after deployment
    --rollback                     Rollback to previous binary version
    --list-deployments             List current deployments on targets
    -v, --verbose                  Enable verbose output
    --dry-run                      Show what would be compiled/deployed

ARGS:
    <EXECUTION_PLAN>  Path to execution plan file (or stdin if -)
```

### Core Data Structures

```rust
// Main deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentPlan {
    pub metadata: DeploymentMetadata,
    pub binary_compilations: Vec<BinaryCompilation>,
    pub deployment_targets: Vec<DeploymentTarget>,
    pub deployment_strategy: DeploymentStrategy,
    pub rollback_info: Option<RollbackInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentMetadata {
    pub deployment_id: String,
    pub created_at: DateTime<Utc>,
    pub rustle_version: String,
    pub execution_plan_hash: String,
    pub compiler_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryCompilation {
    pub compilation_id: String,
    pub binary_name: String,
    pub target_triple: String,
    pub source_tasks: Vec<String>,
    pub embedded_data: EmbeddedExecutionData,
    pub compilation_options: CompilationOptions,
    pub output_path: PathBuf,
    pub checksum: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedExecutionData {
    pub execution_plan: String,
    pub module_implementations: Vec<ModuleImplementation>,
    pub static_files: Vec<StaticFile>,
    pub runtime_config: RuntimeConfig,
    pub facts_template: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleImplementation {
    pub module_name: String,
    pub source_code: String,
    pub dependencies: Vec<String>,
    pub static_linked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticFile {
    pub embedded_path: String,
    pub target_path: String,
    pub content: Vec<u8>,
    pub permissions: u32,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub controller_endpoint: Option<String>,
    pub execution_timeout: Duration,
    pub report_interval: Duration,
    pub cleanup_on_completion: bool,
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentTarget {
    pub host: String,
    pub target_path: String,
    pub binary_compilation_id: String,
    pub deployment_method: DeploymentMethod,
    pub status: DeploymentStatus,
    pub deployed_at: Option<DateTime<Utc>>,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentMethod {
    Ssh,
    Scp,
    Rsync,
    Custom { command: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentStatus {
    Pending,
    Compiling,
    Compiled,
    Deploying,
    Deployed,
    Failed { error: String },
    Verified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentStrategy {
    Parallel,
    Rolling { batch_size: u32 },
    BlueGreen,
    CanaryDeployment { percentage: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationOptions {
    pub optimization_level: OptimizationLevel,
    pub strip_symbols: bool,
    pub static_linking: bool,
    pub compression: bool,
    pub custom_features: Vec<String>,
    pub target_cpu: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationLevel {
    Debug,
    Release,
    ReleaseWithDebugInfo,
    MinSize,
}
```

### Deployment Manager API

```rust
pub struct DeploymentManager {
    config: DeploymentConfig,
    compiler: BinaryCompiler,
    deployer: BinaryDeployer,
    cache: CompilationCache,
}

impl DeploymentManager {
    pub fn new(config: DeploymentConfig) -> Self;
    
    pub async fn create_deployment_plan(
        &self,
        execution_plan: &ExecutionPlan,
        inventory: &ParsedInventory,
    ) -> Result<DeploymentPlan, DeployError>;
    
    pub async fn compile_binaries(
        &self,
        plan: &DeploymentPlan,
    ) -> Result<Vec<BinaryCompilation>, DeployError>;
    
    pub async fn deploy_binaries(
        &self,
        plan: &DeploymentPlan,
    ) -> Result<DeploymentReport, DeployError>;
    
    pub async fn verify_deployments(
        &self,
        targets: &[DeploymentTarget],
    ) -> Result<VerificationReport, DeployError>;
    
    pub async fn cleanup_deployments(
        &self,
        targets: &[DeploymentTarget],
    ) -> Result<(), DeployError>;
    
    pub async fn rollback_deployment(
        &self,
        deployment_id: &str,
    ) -> Result<(), DeployError>;
}

pub struct BinaryCompiler {
    cache: CompilationCache,
    cross_compiler: CrossCompiler,
}

impl BinaryCompiler {
    pub async fn compile_binary(
        &self,
        compilation: &BinaryCompilation,
    ) -> Result<CompiledBinary, CompileError>;
    
    pub fn check_cache(
        &self,
        compilation_hash: &str,
    ) -> Option<CompiledBinary>;
    
    pub async fn cross_compile(
        &self,
        source: &str,
        target_triple: &str,
        options: &CompilationOptions,
    ) -> Result<Vec<u8>, CompileError>;
}

pub struct BinaryDeployer {
    connection_manager: ConnectionManager,
}

impl BinaryDeployer {
    pub async fn deploy_to_host(
        &self,
        binary: &CompiledBinary,
        target: &DeploymentTarget,
    ) -> Result<(), DeployError>;
    
    pub async fn verify_deployment(
        &self,
        target: &DeploymentTarget,
    ) -> Result<bool, DeployError>;
    
    pub async fn execute_binary(
        &self,
        target: &DeploymentTarget,
        args: &[String],
    ) -> Result<ExecutionResult, DeployError>;
}
```

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum DeployError {
    #[error("Compilation failed for target {target}: {reason}")]
    CompilationFailed { target: String, reason: String },
    
    #[error("Cross-compilation not supported for target: {target}")]
    UnsupportedTarget { target: String },
    
    #[error("Deployment failed to host {host}: {reason}")]
    DeploymentFailed { host: String, reason: String },
    
    #[error("Binary verification failed on {host}: expected {expected}, got {actual}")]
    VerificationFailed { host: String, expected: String, actual: String },
    
    #[error("Module {module} not compatible with static linking")]
    StaticLinkingError { module: String },
    
    #[error("Binary size {size} exceeds limit {limit}")]
    BinarySizeExceeded { size: u64, limit: u64 },
    
    #[error("Deployment timeout exceeded: {timeout}s")]
    DeploymentTimeout { timeout: u64 },
    
    #[error("Rollback failed for deployment {deployment_id}: {reason}")]
    RollbackFailed { deployment_id: String, reason: String },
    
    #[error("Cache corruption detected: {path}")]
    CacheCorruption { path: String },
    
    #[error("Insufficient disk space on {host}: required {required}, available {available}")]
    InsufficientSpace { host: String, required: u64, available: u64 },
}
```

## File and Package Structure

```
src/bin/rustle-deploy.rs       # Main binary entry point
src/deploy/
├── mod.rs                     # Module exports
├── manager.rs                 # Deployment management
├── compiler.rs                # Binary compilation
├── deployer.rs                # Binary deployment
├── cache.rs                   # Compilation caching
├── cross_compile.rs           # Cross-compilation support
├── verification.rs            # Deployment verification
├── rollback.rs                # Rollback management
├── template.rs                # Binary template generation
└── error.rs                   # Error types

src/compiler/
├── mod.rs                     # Compiler module exports
├── embedding.rs               # Data embedding
├── optimization.rs            # Binary optimization
├── linking.rs                 # Static linking
└── targets.rs                 # Target platform support

src/types/
├── deployment.rs              # Deployment data structures
└── compilation.rs             # Compilation data structures

tests/deploy/
├── compiler_tests.rs
├── deployer_tests.rs
├── integration_tests.rs
└── cross_compile_tests.rs
```

## Implementation Details

### Phase 1: Basic Compilation
1. Implement core binary compilation from execution plans
2. Create embedded data generation for execution plans and modules
3. Add basic cross-compilation support for x86_64 Linux
4. Implement simple SSH-based deployment

### Phase 2: Optimization and Caching
1. Add compilation caching and incremental builds
2. Implement binary size optimization
3. Add compression and symbol stripping
4. Create compilation dependency tracking

### Phase 3: Advanced Deployment
1. Implement rolling and parallel deployment strategies
2. Add binary verification and integrity checking
3. Create rollback and versioning system
4. Add deployment monitoring and reporting

### Phase 4: Cross-Platform Support
1. Extend cross-compilation to ARM64, macOS, Windows
2. Add target architecture detection from inventory
3. Implement platform-specific optimizations
4. Create automated toolchain management

### Key Algorithms

**Binary Template Generation**:
```rust
fn generate_binary_template(
    compilation: &BinaryCompilation,
) -> Result<String, CompileError> {
    let mut template = String::new();
    
    // Generate main.rs template
    template.push_str(&format!(r#"
use std::collections::HashMap;
use serde_json::Value;

mod embedded_data {{
    use super::*;
    
    pub const EXECUTION_PLAN: &str = r#"{}"#;
    pub const RUNTIME_CONFIG: &str = r#"{}"#;
    
    pub fn get_embedded_files() -> HashMap<String, Vec<u8>> {{
        let mut files = HashMap::new();
        {}
        files
    }}
}}

mod modules {{
    use super::*;
    {}
}}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let execution_plan: ExecutionPlan = serde_json::from_str(embedded_data::EXECUTION_PLAN)?;
    let config: RuntimeConfig = serde_json::from_str(embedded_data::RUNTIME_CONFIG)?;
    
    let executor = LocalExecutor::new(config);
    let result = executor.execute_plan(execution_plan).await?;
    
    if let Some(controller) = config.controller_endpoint {{
        report_to_controller(&controller, &result).await?;
    }}
    
    Ok(())
}}
"#,
        serde_json::to_string(&compilation.embedded_data.execution_plan)?,
        serde_json::to_string(&compilation.embedded_data.runtime_config)?,
        generate_embedded_file_declarations(&compilation.embedded_data.static_files),
        generate_module_implementations(&compilation.embedded_data.module_implementations),
    ));
    
    Ok(template)
}

fn generate_embedded_file_declarations(files: &[StaticFile]) -> String {
    files.iter()
        .map(|file| format!(
            r#"files.insert("{}", include_bytes!("{}").to_vec());"#,
            file.embedded_path,
            file.embedded_path
        ))
        .collect::<Vec<_>>()
        .join("\n        ")
}

fn generate_module_implementations(modules: &[ModuleImplementation]) -> String {
    modules.iter()
        .map(|module| format!(
            r#"
    pub mod {} {{
        use super::*;
        {}
    }}
    "#,
            module.module_name,
            module.source_code
        ))
        .collect::<Vec<_>>()
        .join("\n")
}
```

**Cross-Compilation Management**:
```rust
async fn cross_compile_binary(
    source_dir: &Path,
    target_triple: &str,
    options: &CompilationOptions,
) -> Result<Vec<u8>, CompileError> {
    let mut cmd = tokio::process::Command::new("cargo");
    
    cmd.args(&["build", "--release"])
       .arg("--target")
       .arg(target_triple)
       .current_dir(source_dir);
    
    // Add optimization flags
    match options.optimization_level {
        OptimizationLevel::MinSize => {
            cmd.env("CARGO_TARGET_DIR", "./target-minsize");
            cmd.env("RUSTFLAGS", "-C opt-level=z -C target-cpu=native");
        }
        OptimizationLevel::Release => {
            cmd.env("RUSTFLAGS", "-C opt-level=3 -C target-cpu=native");
        }
        _ => {}
    }
    
    if options.strip_symbols {
        cmd.env("RUSTFLAGS", format!("{} -C strip=symbols", 
                cmd.get_envs().find(|(k, _)| k == "RUSTFLAGS")
                   .map(|(_, v)| v.to_string_lossy())
                   .unwrap_or_default()));
    }
    
    let output = cmd.output().await?;
    
    if !output.status.success() {
        return Err(CompileError::CompilationFailed {
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }
    
    let binary_path = source_dir
        .join("target")
        .join(target_triple)
        .join("release")
        .join("rustle-runner");
    
    let binary_data = tokio::fs::read(&binary_path).await?;
    
    if options.compression {
        compress_binary(&binary_data)
    } else {
        Ok(binary_data)
    }
}

async fn deploy_binary_to_host(
    binary_data: &[u8],
    target: &DeploymentTarget,
    connection: &Connection,
) -> Result<(), DeployError> {
    // Create temporary file for binary
    let temp_path = format!("/tmp/rustle-runner-{}", uuid::Uuid::new_v4());
    
    // Upload binary
    connection.upload_file(binary_data, &temp_path).await?;
    
    // Set executable permissions
    connection.execute_command(&format!("chmod +x {}", temp_path)).await?;
    
    // Move to target location
    connection.execute_command(&format!("mv {} {}", temp_path, target.target_path)).await?;
    
    // Verify deployment
    let checksum_output = connection.execute_command(&format!(
        "sha256sum {} | cut -d' ' -f1", 
        target.target_path
    )).await?;
    
    let deployed_checksum = checksum_output.stdout.trim();
    let expected_checksum = calculate_checksum(binary_data);
    
    if deployed_checksum != expected_checksum {
        return Err(DeployError::VerificationFailed {
            host: target.host.clone(),
            expected: expected_checksum,
            actual: deployed_checksum.to_string(),
        });
    }
    
    Ok(())
}
```

## Testing Strategy

### Unit Tests
- **Compilation**: Test binary template generation and compilation
- **Cross-compilation**: Test different target architectures
- **Embedding**: Test data embedding and extraction
- **Deployment**: Test binary deployment and verification

### Integration Tests
- **End-to-end**: Test complete compile-deploy-execute workflow
- **Multi-platform**: Test cross-compilation for different targets
- **Large deployments**: Test deployment to 100+ hosts
- **Failure scenarios**: Test rollback and recovery

### Test Infrastructure
```
tests/fixtures/
├── execution_plans/
│   ├── simple_plan.json        # Basic execution plan
│   ├── complex_plan.json       # Multi-module execution plan
│   └── large_plan.json         # 1000+ task execution plan
├── binaries/
│   ├── test_targets/           # Pre-compiled test binaries
│   └── checksums/              # Expected checksums
└── deployments/
    ├── single_host.json        # Single host deployment
    └── multi_host.json         # Multi-host deployment
```

## Edge Cases & Error Handling

### Compilation Issues
- Cross-compilation toolchain missing
- Source code generation errors
- Binary size limits exceeded
- Compilation timeout handling

### Deployment Issues
- Network failures during deployment
- Insufficient disk space on targets
- Permission denied on target paths
- Binary corruption during transfer

### Runtime Issues
- Binary execution failures on targets
- Controller communication failures
- Rollback to previous versions
- Partial deployment recovery

## Dependencies

### External Crates
```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive"] }
anyhow = "1"
thiserror = "1"
tracing = "0.1"
uuid = { version = "1", features = ["v4"] }
sha2 = "0.10"
flate2 = "1"
tar = "0.4"
tempfile = "3"

[build-dependencies]
cargo_metadata = "0.18"
```

### Internal Dependencies
- `rustle::types` - Core type definitions
- `rustle::error` - Error handling
- `rustle-connect` - SSH connection management
- Cross-compilation toolchains (rustc, cargo)

## Configuration

### Environment Variables
- `RUSTLE_DEPLOY_CACHE_DIR`: Compilation cache directory
- `RUSTLE_CROSS_COMPILE_ZIG`: Use Zig for cross-compilation
- `RUSTLE_BINARY_SIZE_LIMIT`: Maximum binary size limit
- `RUSTLE_DEPLOYMENT_TIMEOUT`: Default deployment timeout

### Configuration File Support
```toml
[deployment]
cache_dir = "~/.rustle/cache"
output_dir = "./target/deploy"
parallel_jobs = 8
default_timeout_secs = 120
verify_deployments = true

[compilation]
optimization_level = "release"
strip_symbols = true
static_linking = true
compression = true
binary_size_limit_mb = 50

[targets]
default_arch = "x86_64-unknown-linux-gnu"
supported_targets = [
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin"
]

[cross_compilation]
use_zigbuild = true
toolchain_auto_install = true
```

## Documentation

### CLI Help Text
```
rustle-deploy - Compile and deploy optimized execution binaries

USAGE:
    rustle-deploy [OPTIONS] [EXECUTION_PLAN]

ARGS:
    <EXECUTION_PLAN>    Path to execution plan file (or stdin if -)

OPTIONS:
    -i, --inventory <FILE>         Inventory file with target host information
    -o, --output-dir <DIR>         Directory for compiled binaries [default: ./target]
    -t, --target <TRIPLE>          Target architecture (auto-detect from inventory)
        --cache-dir <DIR>          Compilation cache directory
        --incremental              Enable incremental compilation
        --rebuild                  Force rebuild of all binaries
        --deploy-only              Deploy existing binaries without compilation
        --compile-only             Compile binaries without deployment
        --cleanup                  Remove deployed binaries from targets
        --parallel <NUM>           Parallel compilation jobs [default: CPU cores]
        --timeout <SECONDS>        Deployment timeout per host [default: 120]
        --verify                   Verify binary integrity after deployment
    -v, --verbose                  Enable verbose output
        --dry-run                  Show what would be compiled/deployed
    -h, --help                     Print help information
    -V, --version                  Print version information

EXAMPLES:
    rustle-deploy plan.json                           # Compile and deploy binaries
    rustle-deploy --compile-only plan.json           # Only compile binaries
    rustle-deploy --deploy-only plan.json            # Deploy existing binaries
    rustle-deploy --target x86_64-unknown-linux-gnu plan.json  # Specific target
    rustle-deploy --cleanup --inventory hosts.ini    # Clean up deployed binaries
```

### Integration Examples
```bash
# Complete pipeline with binary deployment
rustle-parse playbook.yml | \
  rustle-plan --strategy binary-hybrid | \
  rustle-deploy --verify | \
  rustle-exec

# Incremental deployment with caching
rustle-deploy --incremental --cache-dir ~/.rustle/cache plan.json

# Multi-architecture deployment
rustle-deploy --target x86_64-unknown-linux-gnu plan.json
rustle-deploy --target aarch64-unknown-linux-gnu plan.json

# Development workflow with rapid iteration
rustle-deploy --compile-only --parallel 16 plan.json  # Fast compilation
rustle-deploy --deploy-only plan.json                 # Quick deployment

# Production deployment with verification
rustle-deploy --verify --timeout 300 production-plan.json

# Cleanup after execution
rustle-deploy --cleanup --inventory production.ini
```