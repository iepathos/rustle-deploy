# Spec 110: End-to-End Testing Framework

## Feature Summary

Create a comprehensive end-to-end testing framework for rustle-deploy that validates the complete pipeline from execution plan input to binary compilation and deployment verification. The framework will use containerized environments and realistic test fixtures to ensure the tool works correctly across different platforms, architectures, and deployment scenarios.

**Problem**: Current testing focuses on unit and integration tests but lacks comprehensive end-to-end validation that mirrors real-world usage patterns. We need to verify that:
- Execution plans are correctly compiled into working binaries
- Cross-compilation produces functional binaries for target architectures  
- Generated binaries execute the expected tasks correctly
- Deployment strategies (binary-only, SSH fallback, hybrid) work as intended
- Performance characteristics meet requirements (10x+ improvement over SSH)

**Solution**: Implement a multi-layered E2E testing framework using Docker containers for isolated testing environments, comprehensive JSON fixtures for realistic test scenarios, and automated verification of binary execution results.

## Goals & Requirements

### Functional Requirements

1. **Binary Execution Validation**
   - Verify compiled binaries execute intended tasks correctly
   - Test cross-compiled binaries on their target platforms
   - Validate binary embedding of execution data
   - Ensure binaries work without network dependencies

2. **Deployment Strategy Testing**
   - Test binary-only deployment scenarios
   - Test SSH fallback when binary deployment fails
   - Test hybrid binary+SSH deployment strategies
   - Validate deployment decision logic

3. **Realistic Test Scenarios**
   - Support complex multi-host, multi-architecture plans
   - Test large-scale deployments (100+ hosts)
   - Validate different module types (command, package, service, etc.)
   - Test dependency resolution and execution order

4. **Platform Coverage**
   - Test compilation on Linux, macOS, Windows hosts
   - Test target binaries for x86_64, ARM64 architectures
   - Validate cross-compilation from any host to any target
   - Test container-based and VM-based target environments

### Non-Functional Requirements

1. **Performance Validation**
   - Measure and validate 10x+ performance improvement claims
   - Benchmark compilation times for different plan sizes
   - Monitor binary size and optimization effectiveness
   - Test cache effectiveness for incremental builds

2. **Reliability**
   - Test error handling and recovery scenarios
   - Validate graceful degradation (binary → SSH fallback)
   - Test network failure scenarios
   - Ensure consistent results across test runs

3. **Security**
   - Validate binary integrity and checksums
   - Test secure deployment practices
   - Ensure no credential leakage in binaries
   - Validate sandbox isolation in test environments

### Success Criteria

- [ ] 95%+ of compiled binaries execute successfully on target platforms
- [ ] All supported deployment strategies work correctly
- [ ] Performance benchmarks consistently show 10x+ improvement
- [ ] Test suite completes in under 30 minutes on CI/CD
- [ ] Zero false positives from E2E test failures
- [ ] Support for testing plans with 100+ hosts

## API/Interface Design

### Test Framework Core

```rust
// src/testing/e2e_framework.rs
use std::path::Path;
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct E2ETestConfig {
    pub test_name: String,
    pub execution_plan_path: String,
    pub target_platforms: Vec<TargetPlatform>,
    pub deployment_strategy: DeploymentStrategy,
    pub expected_outcomes: Vec<ExpectedOutcome>,
    pub timeout_seconds: u64,
    pub use_containers: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetPlatform {
    pub arch: String,           // "x86_64", "aarch64"
    pub os: String,             // "linux", "macos", "windows"
    pub environment: TestEnvironment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestEnvironment {
    Docker { image: String, volumes: Vec<String> },
    VM { image: String, snapshot: Option<String> },
    Native,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentStrategy {
    BinaryOnly,
    SSHFallback,
    Hybrid,
    ForceSSH,  // For comparison testing
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedOutcome {
    pub host: String,
    pub tasks: Vec<TaskExpectation>,
    pub performance_threshold: Option<PerformanceThreshold>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExpectation {
    pub task_id: String,
    pub expected_result: TaskResult,
    pub validation_commands: Vec<String>,
    pub artifacts: Vec<ArtifactCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskResult {
    Success,
    Failure { expected_error: String },
    Skip { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactCheck {
    pub path: String,
    pub check_type: ArtifactCheckType,
    pub expected_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactCheckType {
    FileExists,
    FileContent { regex: bool },
    FileMode,
    FileSize { min: Option<u64>, max: Option<u64> },
    ProcessRunning,
    ServiceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThreshold {
    pub max_execution_time_seconds: u64,
    pub max_binary_size_mb: u64,
    pub min_improvement_factor: f64,  // vs SSH baseline
}

pub struct E2ETestRunner {
    config: E2ETestConfig,
    temp_dir: tempfile::TempDir,
    containers: Vec<ContainerHandle>,
}

impl E2ETestRunner {
    pub async fn new(config: E2ETestConfig) -> Result<Self>;
    
    pub async fn setup_test_environment(&mut self) -> Result<()>;
    
    pub async fn compile_execution_plan(&self) -> Result<CompilationResult>;
    
    pub async fn deploy_and_execute(&self, result: CompilationResult) -> Result<ExecutionResult>;
    
    pub async fn validate_outcomes(&self, result: ExecutionResult) -> Result<ValidationReport>;
    
    pub async fn cleanup(&mut self) -> Result<()>;
    
    pub async fn run_full_test(&mut self) -> Result<TestReport>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompilationResult {
    pub binary_path: PathBuf,
    pub compilation_time: Duration,
    pub binary_size: u64,
    pub target_platforms: Vec<TargetPlatform>,
    pub cache_hit: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub deployment_strategy_used: DeploymentStrategy,
    pub execution_time: Duration,
    pub host_results: Vec<HostExecutionResult>,
    pub performance_metrics: PerformanceMetrics,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HostExecutionResult {
    pub host: String,
    pub task_results: Vec<TaskExecutionResult>,
    pub deployment_method: String,
    pub execution_time: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskExecutionResult {
    pub task_id: String,
    pub status: TaskExecutionStatus,
    pub output: String,
    pub execution_time: Duration,
    pub artifacts_created: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TaskExecutionStatus {
    Success,
    Failed { error: String },
    Skipped { reason: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_execution_time: Duration,
    pub ssh_baseline_time: Option<Duration>,
    pub improvement_factor: Option<f64>,
    pub network_requests: u64,
    pub data_transferred: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationReport {
    pub test_name: String,
    pub overall_result: TestResult,
    pub task_validations: Vec<TaskValidationResult>,
    pub performance_validation: PerformanceValidationResult,
    pub deployment_validation: DeploymentValidationResult,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TestResult {
    Pass,
    Fail { reasons: Vec<String> },
    Warning { issues: Vec<String> },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestReport {
    pub config: E2ETestConfig,
    pub compilation_result: CompilationResult,
    pub execution_result: ExecutionResult,
    pub validation_report: ValidationReport,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: chrono::DateTime<chrono::Utc>,
}
```

### Container Management

```rust
// src/testing/containers.rs
use anyhow::Result;
use std::collections::HashMap;

pub struct ContainerManager {
    containers: HashMap<String, ContainerHandle>,
    network: Option<String>,
}

pub struct ContainerHandle {
    pub id: String,
    pub name: String,
    pub ip_address: String,
    pub platform: TargetPlatform,
    pub ssh_port: u16,
}

impl ContainerManager {
    pub async fn new() -> Result<Self>;
    
    pub async fn create_test_network(&mut self) -> Result<String>;
    
    pub async fn start_target_container(
        &mut self, 
        platform: TargetPlatform,
        name: String
    ) -> Result<ContainerHandle>;
    
    pub async fn setup_ssh_access(&self, container: &ContainerHandle) -> Result<()>;
    
    pub async fn copy_binary_to_container(
        &self,
        container: &ContainerHandle,
        binary_path: &Path,
        target_path: &str
    ) -> Result<()>;
    
    pub async fn execute_in_container(
        &self,
        container: &ContainerHandle,
        command: &str
    ) -> Result<String>;
    
    pub async fn cleanup_all(&mut self) -> Result<()>;
}

// GitHub Actions specific utilities
// src/testing/github_actions.rs
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubActionsUtils;

impl GitHubActionsUtils {
    /// Detect if running in GitHub Actions environment
    pub fn is_github_actions() -> bool {
        env::var("GITHUB_ACTIONS").unwrap_or_default() == "true"
    }
    
    /// Get GitHub runner information
    pub fn get_runner_info() -> Option<GitHubRunnerInfo> {
        if !Self::is_github_actions() {
            return None;
        }
        
        Some(GitHubRunnerInfo {
            runner_os: env::var("RUNNER_OS").unwrap_or_default(),
            runner_arch: env::var("RUNNER_ARCH").unwrap_or_default(),
            runner_name: env::var("RUNNER_NAME").unwrap_or_default(),
            is_github_actions: true,
        })
    }
    
    /// Set up GitHub Actions specific environment
    pub async fn setup_github_environment() -> Result<()> {
        // Set up artifact directories
        tokio::fs::create_dir_all("test-logs").await?;
        tokio::fs::create_dir_all("performance-data").await?;
        
        // Configure GitHub Actions specific settings
        if Self::is_github_actions() {
            println!("::group::Setting up E2E test environment");
        }
        
        Ok(())
    }
    
    /// Generate GitHub Actions step outputs
    pub fn set_output(name: &str, value: &str) -> Result<()> {
        if Self::is_github_actions() {
            println!("::set-output name={}::{}", name, value);
        }
        Ok(())
    }
    
    /// Add GitHub Actions annotations
    pub fn add_annotation(level: &str, message: &str, file: Option<&str>, line: Option<u32>) -> Result<()> {
        if Self::is_github_actions() {
            let mut annotation = format!("::{} ::{}", level, message);
            if let Some(f) = file {
                annotation.push_str(&format!(" file={}", f));
            }
            if let Some(l) = line {
                annotation.push_str(&format!(" line={}", l));
            }
            println!("{}", annotation);
        }
        Ok(())
    }
    
    /// Clean up GitHub Actions environment
    pub async fn cleanup_github_environment() -> Result<()> {
        if Self::is_github_actions() {
            println!("::endgroup::");
        }
        Ok(())
    }
    
    /// Determine optimal test strategy based on runner
    pub fn get_optimal_test_strategy() -> TestStrategy {
        if let Some(runner_info) = Self::get_runner_info() {
            match runner_info.runner_os.as_str() {
                "Linux" => TestStrategy::LinuxOptimized,
                "macOS" => TestStrategy::MacOSOptimized,
                "Windows" => TestStrategy::WindowsOptimized,
                _ => TestStrategy::Generic,
            }
        } else {
            TestStrategy::LocalDevelopment
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestStrategy {
    LinuxOptimized,      // Use native compilation + containers
    MacOSOptimized,      // Use native compilation + cross-compilation
    WindowsOptimized,    // Use native compilation + WSL/containers
    Generic,             // Generic GitHub runner
    LocalDevelopment,    // Local development environment
}
```

### Test Fixture Management

```rust
// src/testing/fixtures.rs
use serde_json::Value;
use anyhow::Result;
use crate::testing::github_actions::GitHubActionsUtils;

pub struct TestFixtureManager {
    fixtures_dir: PathBuf,
    github_fixtures_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct TestFixture {
    pub name: String,
    pub execution_plan: Value,
    pub expected_hosts: Vec<String>,
    pub expected_tasks: usize,
    pub description: String,
    pub complexity: ComplexityLevel,
}

#[derive(Debug, Clone)]
pub enum ComplexityLevel {
    Simple,      // Single host, basic tasks
    Medium,      // Multi-host, mixed tasks
    Complex,     // Large scale, dependencies, mixed architectures
    Performance, // Optimized for performance testing
}

impl TestFixtureManager {
    pub fn new<P: AsRef<Path>>(fixtures_dir: P) -> Self;
    
    pub fn load_fixture(&self, name: &str) -> Result<TestFixture>;
    
    pub fn list_fixtures(&self) -> Result<Vec<String>>;
    
    pub fn generate_large_scale_fixture(
        &self,
        host_count: usize,
        platforms: Vec<TargetPlatform>
    ) -> Result<TestFixture>;
    
    pub fn validate_fixture(&self, fixture: &TestFixture) -> Result<()>;
}
```

## File and Package Structure

```
src/
├── testing/                    # New E2E testing framework
│   ├── mod.rs                 # Module declarations
│   ├── e2e_framework.rs       # Core E2E testing framework
│   ├── containers.rs          # Docker container management
│   ├── fixtures.rs            # Test fixture management
│   ├── validation.rs          # Result validation logic
│   ├── performance.rs         # Performance measurement
│   └── reporting.rs           # Test reporting and metrics
├── bin/
│   └── e2e_test_runner.rs     # CLI for running E2E tests
└── ...

tests/
├── e2e/                       # End-to-end tests
│   ├── mod.rs
│   ├── basic_deployment.rs    # Basic deployment scenarios
│   ├── cross_compilation.rs   # Cross-compilation tests
│   ├── large_scale.rs         # Large-scale deployment tests
│   ├── error_scenarios.rs     # Error handling tests
│   └── performance.rs         # Performance benchmarks
├── fixtures/
│   ├── execution_plans/       # Test execution plans
│   │   ├── simple_plan.json          # Single host, basic tasks
│   │   ├── multi_host_plan.json      # Multiple hosts
│   │   ├── cross_platform_plan.json  # Mixed architectures
│   │   ├── large_scale_plan.json     # 100+ hosts
│   │   ├── dependency_plan.json      # Complex dependencies
│   │   ├── windows_plan.json         # Windows-specific tasks
│   │   ├── package_manager_plan.json # Package installation tests
│   │   ├── service_management_plan.json # Service control tests
│   │   └── mixed_modules_plan.json   # All module types
│   └── expected_outcomes/     # Expected test results
│       ├── simple_plan_outcomes.json
│       ├── multi_host_outcomes.json
│       └── ...
└── docker/                   # Docker configurations for testing
    ├── ubuntu-x86_64/        # Ubuntu x86_64 test environment
    │   ├── Dockerfile
    │   └── setup.sh
    ├── ubuntu-arm64/         # Ubuntu ARM64 test environment
    │   ├── Dockerfile
    │   └── setup.sh
    ├── alpine-x86_64/        # Alpine Linux test environment
    │   ├── Dockerfile
    │   └── setup.sh
    └── windows-server/       # Windows Server test environment
        ├── Dockerfile
        └── setup.ps1

.github/
└── workflows/
    └── e2e-tests.yml         # CI/CD workflow for E2E tests

scripts/
├── setup-e2e-env.sh         # Setup script for E2E testing
├── run-e2e-tests.sh         # Script to run all E2E tests
└── cleanup-e2e.sh           # Cleanup script
```

## Implementation Details

### Phase 1: Foundation Setup

1. **Create Test Infrastructure**
```rust
// Implement core E2E framework with container management
// Set up Docker environments for different platforms
// Create basic test fixture loading system
```

2. **Docker Environment Setup**
```dockerfile
# Create standardized test containers with:
# - SSH server configuration
# - Common tools and utilities
# - Platform-specific package managers
# - Monitoring and validation tools
```

3. **Basic Test Fixtures**
```json
// Create comprehensive test execution plans covering:
// - Single host scenarios
// - Multi-host deployments
// - Different module types
// - Various complexity levels
```

### Phase 2: Core E2E Testing

1. **Binary Compilation Testing**
```rust
impl E2ETestRunner {
    async fn test_compilation_pipeline(&self) -> Result<()> {
        // Test execution plan parsing
        // Verify template generation
        // Test binary compilation
        // Validate cross-compilation
        // Check binary integrity
    }
}
```

2. **Deployment Strategy Testing**
```rust
impl E2ETestRunner {
    async fn test_deployment_strategies(&self) -> Result<()> {
        // Test binary-only deployment
        // Test SSH fallback scenarios
        // Test hybrid deployments
        // Validate deployment decision logic
    }
}
```

3. **Binary Execution Validation**
```rust
impl E2ETestRunner {
    async fn validate_binary_execution(&self) -> Result<()> {
        // Execute compiled binaries on target platforms
        // Verify task execution results
        // Check artifact creation
        // Validate output and side effects
    }
}
```

### Phase 3: Advanced Testing Scenarios

1. **Large-Scale Testing**
```rust
// Generate test plans with 100+ hosts
// Test compilation scalability
// Measure performance characteristics
// Validate resource usage
```

2. **Error Scenario Testing**
```rust
// Test compilation failures
// Network failure simulation
// Target host failures
// Recovery and fallback testing
```

3. **Performance Benchmarking**
```rust
impl PerformanceTester {
    async fn benchmark_vs_ssh(&self) -> Result<PerformanceComparison> {
        // Run same plan with binary deployment
        // Run same plan with SSH-only deployment
        // Calculate improvement factors
        // Generate performance reports
    }
}
```

### Phase 4: Integration and Automation

1. **GitHub Actions Integration**
```yaml
# Comprehensive GitHub Actions workflows for:
# - Native multi-platform testing (ubuntu, macos, windows)
# - Multi-architecture support (x86_64, ARM64)
# - Matrix testing strategies
# - Performance regression detection
# - Artifact collection and reporting
# - Parallel test execution across runners
# - Cost-effective use of free runner minutes
```

2. **Test Reporting and Metrics**
```rust
// Generate comprehensive test reports
// Track performance trends over time
// Failure analysis and debugging tools
// Integration with monitoring systems
```

## Testing Strategy

### Unit Tests

1. **Test Framework Components**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_container_management() {
        // Test container lifecycle
        // Test SSH setup
        // Test binary deployment
    }

    #[test]
    fn test_fixture_loading() {
        // Test fixture parsing
        // Test validation logic
        // Test fixture generation
    }

    #[test]
    fn test_result_validation() {
        // Test outcome validation
        // Test performance measurement
        // Test reporting generation
    }
}
```

### Integration Tests

1. **Component Integration**
```rust
#[tokio::test]
async fn test_full_e2e_pipeline() {
    // Test complete E2E workflow
    // Test with real Docker containers
    // Test cross-platform compilation
    // Test deployment and validation
}
```

2. **Performance Integration**
```rust
#[tokio::test]
async fn test_performance_benchmarks() {
    // Test performance measurement accuracy
    // Test baseline comparisons
    // Test improvement factor calculations
}
```

### E2E Test Categories

1. **Basic Functionality Tests**
   - Simple single-host deployments
   - Basic module types (command, package, service)
   - Cross-compilation verification

2. **Advanced Scenario Tests**
   - Multi-host, multi-architecture deployments
   - Complex dependency chains
   - Large-scale deployments (100+ hosts)

3. **Error Recovery Tests**
   - Compilation failure scenarios
   - Network failure simulation
   - SSH fallback testing

4. **Performance Tests**
   - Binary vs SSH performance comparison
   - Compilation time benchmarks
   - Binary size optimization verification

## Edge Cases & Error Handling

### Compilation Failures

1. **Invalid Execution Plans**
```rust
// Handle malformed JSON
// Missing required fields
// Invalid module configurations
// Circular dependencies
```

2. **Cross-Compilation Issues**
```rust
// Missing target toolchains
// Platform-specific compilation errors
// Architecture mismatch detection
```

### Deployment Failures

1. **Network Issues**
```rust
// Connection timeouts
// Authentication failures
// SSH key issues
// Network partitions
```

2. **Target Platform Issues**
```rust
// Incompatible binaries
// Missing dependencies
// Permission issues
// Resource constraints
```

### Validation Failures

1. **Execution Issues**
```rust
// Task execution failures
// Unexpected side effects
// Performance degradation
// Resource leaks
```

2. **Environment Issues**
```rust
// Container startup failures
// Docker daemon issues
// Resource exhaustion
// Platform compatibility
```

## Dependencies

### External Crates

```toml
[dev-dependencies]
# Testing framework
tokio-test = "0.4"
tempfile = "3.0"
uuid = { version = "1.0", features = ["v4"] }

# GitHub Actions integration
octocrab = "0.32"        # GitHub API client (for GitHub Actions integration)
serde_yaml = "0.9"       # YAML parsing for GitHub Actions workflows

# Docker integration (fallback)
bollard = "0.14"          # Docker API client
testcontainers = "0.14"   # Container testing framework

# Performance measurement
criterion = "0.5"         # Benchmarking
sysinfo = "0.29"         # System information

# JSON processing and validation
serde_json = "1.0"
jsonschema = "0.17"      # JSON schema validation

# Time and duration handling
chrono = { version = "0.4", features = ["serde"] }

# Async utilities
futures = "0.3"
tokio = { version = "1.0", features = ["full"] }

# Process management
nix = "0.26"             # Unix process utilities
winapi = "0.3"           # Windows API bindings

# Environment detection
which = "4.4"            # Executable detection
dirs = "5.0"             # Standard directories
```

### Internal Dependencies

- `rustle_deploy::compilation` - Binary compilation functionality
- `rustle_deploy::execution` - Execution plan processing
- `rustle_deploy::deploy` - Deployment management
- `rustle_deploy::runtime` - Runtime execution engine

### System Dependencies

**GitHub Actions Runners (Automatically Available)**:
- **Rust toolchain** - Pre-installed on all GitHub runners
- **Git** - Pre-installed for repository operations
- **Cross-compilation support** - Available through rustup
- **Container runtime** - Docker available on Linux/Windows runners

**Additional Dependencies (Auto-installed)**:
- **Cross-compilation toolchains** - Installed via GitHub Actions workflows
- **Platform-specific tools** - GCC, MSVC, Xcode command line tools
- **SSH client** - Available on all runner platforms

**Fallback Dependencies (For local development)**:
- **Docker** - For containerized testing when not on GitHub Actions
- **SSH client** - For SSH-based deployment testing
- **Git** - For fixture management

## GitHub Actions Advantages

### Why GitHub Actions for E2E Testing

**Open Source Project Benefits**:
- **Free runner minutes**: 2,000 minutes/month for public repositories
- **Native multi-platform support**: Ubuntu, macOS, Windows runners available
- **Multi-architecture support**: x86_64 and ARM64 (M1) runners
- **No infrastructure management**: No need to maintain test servers or containers
- **Parallel execution**: Run tests across multiple platforms simultaneously
- **Artifact storage**: Automatic storage and sharing of test results
- **Integration**: Native integration with GitHub repository and PR workflows

### Platform Coverage with GitHub Runners

```yaml
# Native platform testing without emulation
matrix:
  include:
    # Linux testing
    - os: ubuntu-latest        # x86_64 Linux
    - os: ubuntu-latest-arm64  # ARM64 Linux (if available)
    
    # macOS testing  
    - os: macos-latest         # x86_64 macOS
    - os: macos-latest-arm64   # ARM64 macOS (M1/M2)
    
    # Windows testing
    - os: windows-latest       # x86_64 Windows
```

### Cost-Effective Testing Strategy

1. **Primary Testing**: Use GitHub Actions runners for comprehensive platform coverage
2. **Extended Testing**: Use containers for additional Linux distributions
3. **Performance Testing**: Leverage consistent GitHub runner specs for benchmarking
4. **Nightly Testing**: Run comprehensive tests during off-peak hours

### Hybrid Approach Benefits

- **GitHub Runners**: Native platform testing, cross-compilation validation
- **Docker Containers**: Additional Linux distro testing, isolated environments
- **Combined Coverage**: Maximum platform coverage with minimal infrastructure cost

## Configuration

### E2E Test Configuration

```toml
# tests/e2e.toml
[e2e_testing]
default_timeout_seconds = 300
max_parallel_tests = 4
cleanup_on_failure = true
preserve_artifacts = true
prefer_github_runners = true  # Use GitHub runners when available

[github_actions]
# GitHub Actions specific configuration
use_matrix_strategy = true
max_parallel_jobs = 6        # Respect GitHub's concurrent job limits
store_artifacts = true
cache_dependencies = true
fail_fast = false           # Continue testing other platforms on failure

[docker]
network_name = "rustle-deploy-e2e"
image_pull_policy = "IfNotPresent"
cleanup_containers = true
# Use containers as fallback when GitHub runners unavailable
fallback_to_containers = true

[performance]
baseline_ssh_timeout = 600
min_improvement_factor = 5.0
max_binary_size_mb = 50
benchmark_on_all_platforms = true

[platforms]
# GitHub Actions native runners
github_runners = [
    { os = "ubuntu-latest", arch = "x86_64", target = "x86_64-unknown-linux-gnu" },
    { os = "ubuntu-latest-arm64", arch = "aarch64", target = "aarch64-unknown-linux-gnu" },
    { os = "macos-latest", arch = "x86_64", target = "x86_64-apple-darwin" },
    { os = "macos-latest-arm64", arch = "aarch64", target = "aarch64-apple-darwin" },
    { os = "windows-latest", arch = "x86_64", target = "x86_64-pc-windows-msvc" },
]

# Fallback container targets
container_targets = [
    { arch = "x86_64", os = "linux", image = "ubuntu:22.04" },
    { arch = "aarch64", os = "linux", image = "arm64v8/ubuntu:22.04" },
]

[fixtures]
fixtures_dir = "tests/fixtures"
generate_large_scale = true
max_hosts_for_testing = 100
# GitHub runner specific fixtures
github_runner_fixtures = "tests/fixtures/github_runners"
```

### Environment Variables

```bash
# GitHub Actions detection
GITHUB_ACTIONS=true          # Set automatically by GitHub Actions
RUNNER_OS=Linux              # Set automatically: Linux, macOS, Windows
RUNNER_ARCH=X64              # Set automatically: X64, ARM64
RUNNER_NAME=GitHub_Actions   # Set automatically

# E2E Testing configuration
RUSTLE_E2E_USE_GITHUB_RUNNERS=true  # Prefer GitHub runners over containers
RUSTLE_E2E_MATRIX_TESTING=true      # Enable matrix testing strategy
RUSTLE_E2E_PARALLEL_JOBS=6          # Max parallel jobs (GitHub limit)

# Container fallback configuration
RUSTLE_E2E_DOCKER_NETWORK=rustle-deploy-e2e
RUSTLE_E2E_CONTAINER_TIMEOUT=300
RUSTLE_E2E_PRESERVE_CONTAINERS=false
RUSTLE_E2E_FALLBACK_TO_CONTAINERS=true

# Test configuration
RUSTLE_E2E_PARALLEL_TESTS=4
RUSTLE_E2E_CLEANUP_ON_FAILURE=true
RUSTLE_E2E_PERFORMANCE_BASELINE=true
RUSTLE_E2E_BENCHMARK_ALL_PLATFORMS=true

# Fixture configuration
RUSTLE_E2E_FIXTURES_DIR=tests/fixtures
RUSTLE_E2E_GITHUB_FIXTURES_DIR=tests/fixtures/github_runners
RUSTLE_E2E_GENERATE_FIXTURES=true
RUSTLE_E2E_MAX_TEST_HOSTS=100

# Artifact and caching
RUSTLE_E2E_STORE_ARTIFACTS=true
RUSTLE_E2E_CACHE_DEPENDENCIES=true
RUSTLE_E2E_ARTIFACT_RETENTION_DAYS=30
```

## Documentation

### CLI Integration

```bash
# New binary for E2E testing
cargo build --bin e2e_test_runner

# GitHub Actions native testing
./target/debug/e2e_test_runner --github-runner --native-platform
./target/debug/e2e_test_runner --github-runner --matrix-all

# Platform-specific testing on GitHub runners
./target/debug/e2e_test_runner --platform ubuntu-latest --arch x86_64
./target/debug/e2e_test_runner --platform macos-latest-arm64 --arch aarch64
./target/debug/e2e_test_runner --platform windows-latest --arch x86_64

# Cross-compilation testing
./target/debug/e2e_test_runner --category cross-compilation --source-platform x86_64
./target/debug/e2e_test_runner --cross-compile-all --from-github-runner

# Container fallback testing
./target/debug/e2e_test_runner --category container --fallback-mode
./target/debug/e2e_test_runner --container-image ubuntu:22.04

# Performance benchmarking
./target/debug/e2e_test_runner --category performance --benchmark-vs-ssh
./target/debug/e2e_test_runner --large-scale-test --github-runners-only

# Run with specific fixtures
./target/debug/e2e_test_runner --fixture github_runners/ubuntu_native.json
./target/debug/e2e_test_runner --fixture github_runners/cross_platform.json

# Generate test reports with GitHub Actions integration
./target/debug/e2e_test_runner --all --report-format json --output results.json --github-actions
./target/debug/e2e_test_runner --matrix-all --report-format html --output matrix-report.html

# GitHub Actions specific commands
./target/debug/e2e_test_runner --detect-runner-info
./target/debug/e2e_test_runner --setup-github-runner
./target/debug/e2e_test_runner --cleanup-github-artifacts
```

### Usage Examples

1. **Basic E2E Test**
```rust
use rustle_deploy::testing::E2ETestRunner;

#[tokio::main]
async fn main() -> Result<()> {
    let config = E2ETestConfig {
        test_name: "basic_deployment".to_string(),
        execution_plan_path: "tests/fixtures/simple_plan.json".to_string(),
        target_platforms: vec![
            TargetPlatform {
                arch: "x86_64".to_string(),
                os: "linux".to_string(),
                environment: TestEnvironment::Docker {
                    image: "ubuntu:22.04".to_string(),
                    volumes: vec![],
                },
            }
        ],
        deployment_strategy: DeploymentStrategy::BinaryOnly,
        expected_outcomes: vec![/* ... */],
        timeout_seconds: 300,
        use_containers: true,
    };

    let mut runner = E2ETestRunner::new(config).await?;
    let report = runner.run_full_test().await?;
    
    println!("Test result: {:?}", report.validation_report.overall_result);
    Ok(())
}
```

2. **Performance Comparison Test**
```rust
// Compare binary deployment vs SSH deployment
let binary_config = E2ETestConfig {
    deployment_strategy: DeploymentStrategy::BinaryOnly,
    // ... other config
};

let ssh_config = E2ETestConfig {
    deployment_strategy: DeploymentStrategy::ForceSSH,
    // ... other config
};

let binary_result = E2ETestRunner::new(binary_config).await?.run_full_test().await?;
let ssh_result = E2ETestRunner::new(ssh_config).await?.run_full_test().await?;

let improvement = binary_result.execution_result.performance_metrics.total_execution_time.as_secs_f64() 
    / ssh_result.execution_result.performance_metrics.total_execution_time.as_secs_f64();

println!("Performance improvement: {:.2}x", improvement);
```

### Integration Points

1. **GitHub Actions Integration**
```yaml
# .github/workflows/e2e-matrix.yml
name: E2E Matrix Testing
on: [push, pull_request]

jobs:
  # Matrix testing across all supported platforms
  e2e-matrix:
    strategy:
      fail-fast: false
      matrix:
        include:
          # Linux runners
          - os: ubuntu-latest
            arch: x86_64
            target: x86_64-unknown-linux-gnu
          - os: ubuntu-latest-arm64  # If available
            arch: aarch64
            target: aarch64-unknown-linux-gnu
          
          # macOS runners
          - os: macos-latest
            arch: x86_64
            target: x86_64-apple-darwin
          - os: macos-latest-arm64   # M1 runners
            arch: aarch64
            target: aarch64-apple-darwin
          
          # Windows runners
          - os: windows-latest
            arch: x86_64
            target: x86_64-pc-windows-msvc
    
    runs-on: ${{ matrix.os }}
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: ${{ matrix.target }}
          override: true
      
      - name: Cache Cargo
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-${{ matrix.arch }}-cargo-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Install Cross-compilation Tools
        run: |
          # Platform-specific cross-compilation setup
          if [[ "${{ matrix.os }}" == "ubuntu"* ]]; then
            sudo apt-get update
            sudo apt-get install -y gcc-aarch64-linux-gnu
          elif [[ "${{ matrix.os }}" == "macos"* ]]; then
            # macOS cross-compilation setup
            echo "macOS cross-compilation setup"
          fi
        shell: bash
      
      - name: Build E2E Test Runner
        run: cargo build --bin e2e_test_runner --target ${{ matrix.target }}
      
      - name: Run Native Platform Tests
        run: |
          # Test compilation and execution on native platform
          ./target/${{ matrix.target }}/debug/e2e_test_runner \
            --category native \
            --platform ${{ matrix.arch }} \
            --os ${{ runner.os }} \
            --report-format json \
            --output e2e-results-${{ matrix.os }}-${{ matrix.arch }}.json
        shell: bash
      
      - name: Test Cross-compilation
        run: |
          # Test cross-compilation to other targets
          ./target/${{ matrix.target }}/debug/e2e_test_runner \
            --category cross-compilation \
            --source-platform ${{ matrix.arch }} \
            --report-format json \
            --output cross-compile-results-${{ matrix.os }}-${{ matrix.arch }}.json
        shell: bash
      
      - name: Upload Test Results
        uses: actions/upload-artifact@v4
        if: always()
        with:
          name: e2e-results-${{ matrix.os }}-${{ matrix.arch }}
          path: |
            e2e-results-*.json
            cross-compile-results-*.json
            test-logs/
  
  # Performance benchmarking job
  performance-benchmarks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
      
      - name: Setup Docker
        uses: docker/setup-buildx-action@v3
      
      - name: Run Performance Tests
        run: |
          cargo build --bin e2e_test_runner --release
          ./target/release/e2e_test_runner \
            --category performance \
            --benchmark-vs-ssh \
            --large-scale-test \
            --report-format json \
            --output performance-results.json
      
      - name: Upload Performance Results
        uses: actions/upload-artifact@v4
        with:
          name: performance-benchmarks
          path: performance-results.json
  
  # Container-based testing for additional coverage
  container-tests:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        container:
          - ubuntu:22.04
          - alpine:latest
          - debian:bullseye
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Docker
        uses: docker/setup-buildx-action@v3
      
      - name: Run Container Tests
        run: |
          # Test binary execution in different container environments
          cargo build --bin e2e_test_runner
          ./target/debug/e2e_test_runner \
            --category container \
            --container-image ${{ matrix.container }} \
            --report-format json \
            --output container-results-$(echo ${{ matrix.container }} | tr ':' '-').json
      
      - name: Upload Container Results
        uses: actions/upload-artifact@v4
        with:
          name: container-results-${{ matrix.container }}
          path: container-results-*.json

# .github/workflows/e2e-nightly.yml - Comprehensive nightly testing
name: Nightly E2E Tests
on:
  schedule:
    - cron: '0 2 * * *'  # Run at 2 AM UTC daily
  workflow_dispatch:     # Allow manual triggering

jobs:
  comprehensive-testing:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
      
      - name: Run Comprehensive Tests
        run: |
          cargo build --bin e2e_test_runner --release
          # Run all test categories with extended timeouts
          ./target/release/e2e_test_runner \
            --all \
            --large-scale \
            --stress-test \
            --timeout 3600 \
            --report-format html \
            --output nightly-report.html
      
      - name: Upload Comprehensive Results
        uses: actions/upload-artifact@v4
        with:
          name: nightly-comprehensive-results
          path: |
            nightly-report.html
            test-logs/
            performance-data/
```

2. **Performance Monitoring**
```rust
// Integration with monitoring systems
// Track performance trends over time
// Alert on performance regressions
// Generate performance dashboards
```

This comprehensive E2E testing framework provides robust validation of the entire rustle-deploy pipeline, ensuring reliability and performance across all supported platforms and deployment scenarios.