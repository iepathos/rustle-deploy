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
```

### Test Fixture Management

```rust
// src/testing/fixtures.rs
use serde_json::Value;
use anyhow::Result;

pub struct TestFixtureManager {
    fixtures_dir: PathBuf,
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

1. **CI/CD Integration**
```yaml
# GitHub Actions workflow for:
# - Multi-platform testing
# - Performance regression detection
# - Artifact collection and reporting
# - Parallel test execution
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

# Docker integration
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
```

### Internal Dependencies

- `rustle_deploy::compilation` - Binary compilation functionality
- `rustle_deploy::execution` - Execution plan processing
- `rustle_deploy::deploy` - Deployment management
- `rustle_deploy::runtime` - Runtime execution engine

### System Dependencies

- **Docker** - For containerized testing environments
- **SSH client** - For SSH-based deployment testing
- **Cross-compilation toolchains** - For testing target architectures
- **Git** - For fixture management and CI/CD integration

## Configuration

### E2E Test Configuration

```toml
# tests/e2e.toml
[e2e_testing]
default_timeout_seconds = 300
max_parallel_tests = 4
cleanup_on_failure = true
preserve_artifacts = true

[docker]
network_name = "rustle-deploy-e2e"
image_pull_policy = "IfNotPresent"
cleanup_containers = true

[performance]
baseline_ssh_timeout = 600
min_improvement_factor = 5.0
max_binary_size_mb = 50

[platforms]
default_targets = [
    { arch = "x86_64", os = "linux" },
    { arch = "aarch64", os = "linux" },
    { arch = "x86_64", os = "macos" },
]

[fixtures]
fixtures_dir = "tests/fixtures"
generate_large_scale = true
max_hosts_for_testing = 100
```

### Environment Variables

```bash
# Container configuration
RUSTLE_E2E_DOCKER_NETWORK=rustle-deploy-e2e
RUSTLE_E2E_CONTAINER_TIMEOUT=300
RUSTLE_E2E_PRESERVE_CONTAINERS=false

# Test configuration
RUSTLE_E2E_PARALLEL_TESTS=4
RUSTLE_E2E_CLEANUP_ON_FAILURE=true
RUSTLE_E2E_PERFORMANCE_BASELINE=true

# Fixture configuration
RUSTLE_E2E_FIXTURES_DIR=tests/fixtures
RUSTLE_E2E_GENERATE_FIXTURES=true
RUSTLE_E2E_MAX_TEST_HOSTS=100
```

## Documentation

### CLI Integration

```bash
# New binary for E2E testing
cargo build --bin e2e_test_runner

# Run all E2E tests
./target/debug/e2e_test_runner --all

# Run specific test categories
./target/debug/e2e_test_runner --category performance
./target/debug/e2e_test_runner --category cross-compilation
./target/debug/e2e_test_runner --category large-scale

# Run with specific fixtures
./target/debug/e2e_test_runner --fixture simple_plan.json
./target/debug/e2e_test_runner --fixture large_scale_plan.json

# Generate test reports
./target/debug/e2e_test_runner --all --report-format json --output results.json
./target/debug/e2e_test_runner --all --report-format html --output report.html

# Setup and cleanup
./target/debug/e2e_test_runner --setup-only
./target/debug/e2e_test_runner --cleanup-only
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

1. **CI/CD Integration**
```yaml
# .github/workflows/e2e-tests.yml
name: E2E Tests
on: [push, pull_request]

jobs:
  e2e-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
      - name: Setup Docker
        uses: docker/setup-buildx-action@v2
      - name: Run E2E Tests
        run: |
          cargo build --bin e2e_test_runner
          ./target/debug/e2e_test_runner --all --report-format json --output e2e-results.json
      - name: Upload Results
        uses: actions/upload-artifact@v3
        with:
          name: e2e-test-results
          path: e2e-results.json
```

2. **Performance Monitoring**
```rust
// Integration with monitoring systems
// Track performance trends over time
// Alert on performance regressions
// Generate performance dashboards
```

This comprehensive E2E testing framework provides robust validation of the entire rustle-deploy pipeline, ensuring reliability and performance across all supported platforms and deployment scenarios.