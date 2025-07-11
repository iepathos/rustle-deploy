# Spec 090: Zero-Infrastructure Cross-Compilation

## Feature Summary

Implement a zero-infrastructure cross-compilation system that enables rustle-deploy to compile binaries for multiple target architectures without requiring Docker, cloud services, or complex setup. This system uses Zig-based cross-compilation to provide a drop-in Ansible replacement experience with automatic optimization and graceful fallback to SSH deployment.

**Problem it solves**: Current cross-compilation approaches require Docker, remote build infrastructure, or platform-specific toolchains, making rustle-deploy difficult to adopt as a zero-config Ansible replacement. Users need a tool that works out-of-the-box with major performance gains without infrastructure overhead.

**High-level approach**: Leverage cargo-zigbuild for cross-compilation, implement automatic capability detection, and provide intelligent deployment strategies that optimize for binary deployment when possible while gracefully falling back to SSH when cross-compilation is unavailable.

## Goals & Requirements

### Functional Requirements
- Zero-infrastructure cross-compilation using Zig/cargo-zigbuild
- Automatic detection of cross-compilation capabilities
- Graceful fallback to SSH deployment when binary compilation unavailable
- Drop-in replacement for ansible-playbook command syntax
- Intelligent optimization based on target analysis and compilation capabilities
- Support for all major platforms: Linux (x86_64, ARM64), macOS (x86_64, ARM64), Windows (x86_64)
- Transparent user experience with clear feedback on deployment methods used
- Offline operation after initial tool installation
- Automatic binary caching and reuse for identical execution plans

### Non-functional Requirements
- **Ease of Use**: Single command installation, zero configuration required
- **Performance**: 2-10x faster than Ansible for compatible workloads
- **Reliability**: 99%+ deployment success with automatic fallback strategies  
- **Compatibility**: Support all common Ansible modules and playbook patterns
- **Portability**: Works on Linux, macOS, and Windows development machines
- **Efficiency**: Minimize compilation time through intelligent caching and optimization

### Success Criteria
- Users can replace `ansible-playbook` with `rustle-deploy` with identical syntax
- Cross-compilation works without Docker or cloud services when Zig is available
- Automatic capability detection guides users to optimal setup
- Performance gains are measurable and significant for target workloads
- Fallback to SSH maintains 100% Ansible compatibility
- Installation and first-run experience require minimal user intervention

## API/Interface Design

### Core Cross-Compilation System

```rust
/// Zero-infrastructure cross-compilation manager
pub struct ZeroInfraCompiler {
    capabilities: CompilationCapabilities,
    cache: CompilationCache,
    optimizer: DeploymentOptimizer,
}

impl ZeroInfraCompiler {
    pub fn detect_capabilities() -> Self;
    
    pub async fn compile_or_fallback(
        &self,
        template: &GeneratedTemplate,
        inventory: &ParsedInventory,
    ) -> Result<DeploymentPlan, CompilationError>;
    
    pub async fn compile_with_zigbuild(
        &self,
        template: &GeneratedTemplate,
        target: &TargetSpecification,
    ) -> Result<CompiledBinary, ZigCompilationError>;
    
    pub fn get_available_targets(&self) -> &[String];
    
    pub fn requires_fallback(&self, target: &str) -> bool;
    
    pub async fn validate_toolchain(&self) -> Result<ToolchainStatus, ValidationError>;
}

/// Compilation capabilities detection
#[derive(Debug, Clone)]
pub struct CompilationCapabilities {
    pub rust_version: Option<String>,
    pub zig_available: bool,
    pub zigbuild_available: bool,
    pub available_targets: HashSet<String>,
    pub native_target: String,
    pub capability_level: CapabilityLevel,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CapabilityLevel {
    Full,           // Zig + cargo-zigbuild available, all targets supported
    Limited,        // Rust only, native target and some cross-compilation
    Minimal,        // Rust only, native target only
    Insufficient,   // Missing requirements
}

#[derive(Debug, Clone)]
pub struct TargetSpecification {
    pub triple: String,
    pub platform: Platform,
    pub architecture: Architecture,
    pub requires_zig: bool,
    pub compilation_strategy: CompilationStrategy,
}

#[derive(Debug, Clone)]
pub enum CompilationStrategy {
    ZigBuild,
    NativeCargo,
    SshFallback,
}

/// Deployment plan with mixed strategies
#[derive(Debug, Clone)]
pub struct DeploymentPlan {
    pub binary_deployments: Vec<BinaryDeployment>,
    pub ssh_deployments: Vec<SshDeployment>,
    pub estimated_performance_gain: f32,
    pub compilation_time: Duration,
    pub total_targets: usize,
}

#[derive(Debug, Clone)]
pub struct BinaryDeployment {
    pub binary: CompiledBinary,
    pub target_hosts: Vec<String>,
    pub deployment_method: BinaryDeploymentMethod,
}

#[derive(Debug, Clone)]
pub enum BinaryDeploymentMethod {
    DirectExecution,
    UploadAndExecute,
    CachedExecution,
}

#[derive(Debug, Clone)]
pub struct SshDeployment {
    pub execution_plan: ExecutionPlan,
    pub target_hosts: Vec<String>,
    pub fallback_reason: FallbackReason,
}

#[derive(Debug, Clone)]
pub enum FallbackReason {
    UnsupportedTarget,
    CompilationFailure,
    ModuleIncompatibility,
    UserPreference,
}
```

### Capability Detection and Toolchain Management

```rust
/// Detects and manages cross-compilation toolchain
pub struct ToolchainDetector {
    cache: DetectionCache,
}

impl ToolchainDetector {
    pub fn new() -> Self;
    
    pub async fn detect_full_capabilities() -> Result<CompilationCapabilities, DetectionError>;
    
    pub fn check_rust_installation(&self) -> Result<RustInstallation, ToolchainError>;
    
    pub fn check_zig_installation(&self) -> Result<Option<ZigInstallation>, ToolchainError>;
    
    pub fn check_zigbuild_installation(&self) -> Result<bool, ToolchainError>;
    
    pub async fn install_zigbuild_if_missing(&self) -> Result<(), InstallationError>;
    
    pub fn get_supported_targets(&self, capabilities: &CompilationCapabilities) -> Vec<TargetSpecification>;
    
    pub fn recommend_setup_improvements(&self, capabilities: &CompilationCapabilities) -> Vec<SetupRecommendation>;
}

#[derive(Debug, Clone)]
pub struct RustInstallation {
    pub version: String,
    pub toolchain: String,
    pub targets: Vec<String>,
    pub cargo_path: PathBuf,
    pub rustc_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ZigInstallation {
    pub version: String,
    pub zig_path: PathBuf,
    pub supports_cross_compilation: bool,
}

#[derive(Debug, Clone)]
pub struct SetupRecommendation {
    pub improvement: String,
    pub impact: ImpactLevel,
    pub installation_command: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone)]
pub enum ImpactLevel {
    Critical,   // Required for basic functionality
    High,       // Enables cross-compilation
    Medium,     // Performance improvement
    Low,        // Nice to have
}
```

### Deployment Optimizer

```rust
/// Analyzes execution plans and determines optimal deployment strategies
pub struct DeploymentOptimizer {
    binary_analyzer: BinaryDeploymentAnalyzer,
    performance_predictor: PerformancePredictor,
}

impl DeploymentOptimizer {
    pub fn new() -> Self;
    
    pub async fn analyze_optimization_potential(
        &self,
        execution_plan: &RustlePlanOutput,
        capabilities: &CompilationCapabilities,
    ) -> Result<OptimizationAnalysis, AnalysisError>;
    
    pub async fn create_optimal_deployment_plan(
        &self,
        execution_plan: &RustlePlanOutput,
        capabilities: &CompilationCapabilities,
    ) -> Result<DeploymentPlan, OptimizationError>;
    
    pub fn estimate_performance_gain(
        &self,
        binary_tasks: usize,
        ssh_tasks: usize,
        target_hosts: usize,
    ) -> f32;
    
    pub fn should_use_binary_deployment(
        &self,
        tasks: &[TaskPlan],
        target: &TargetSpecification,
    ) -> BinaryDeploymentDecision;
}

#[derive(Debug, Clone)]
pub struct OptimizationAnalysis {
    pub optimization_score: f32,           // 0.0 to 1.0
    pub binary_compatible_tasks: usize,
    pub total_tasks: usize,
    pub estimated_speedup: f32,
    pub compilation_overhead: Duration,
    pub recommended_strategy: RecommendedStrategy,
    pub target_breakdown: HashMap<String, TargetAnalysis>,
}

#[derive(Debug, Clone)]
pub enum RecommendedStrategy {
    BinaryOnly,
    Hybrid,
    SshOnly,
}

#[derive(Debug, Clone)]
pub struct TargetAnalysis {
    pub target_triple: String,
    pub host_count: usize,
    pub compatible_tasks: usize,
    pub compilation_feasible: bool,
    pub estimated_benefit: f32,
}

#[derive(Debug, Clone)]
pub enum BinaryDeploymentDecision {
    Recommended { confidence: f32 },
    Feasible { limitations: Vec<String> },
    NotRecommended { reasons: Vec<String> },
}
```

### Command Line Interface

```rust
/// Main rustle-deploy CLI interface
pub struct RustleDeployCli {
    config: CliConfig,
    capabilities: CompilationCapabilities,
    compiler: ZeroInfraCompiler,
}

impl RustleDeployCli {
    pub fn new() -> Result<Self, InitializationError>;
    
    pub async fn execute_deployment(
        &self,
        playbook: &Path,
        inventory: &Path,
        options: DeployOptions,
    ) -> Result<DeploymentResult, DeploymentError>;
    
    pub async fn check_capabilities(&self) -> Result<CapabilityReport, CheckError>;
    
    pub async fn install_dependencies(&self, components: &[Component]) -> Result<(), InstallationError>;
    
    pub fn print_optimization_report(&self, analysis: &OptimizationAnalysis);
    
    pub fn print_deployment_summary(&self, result: &DeploymentResult);
}

#[derive(Debug, Clone)]
pub struct DeployOptions {
    pub optimization_mode: OptimizationMode,
    pub force_binary: bool,
    pub force_ssh: bool,
    pub verbosity: u8,
    pub dry_run: bool,
    pub cache_binaries: bool,
    pub parallel_compilation: bool,
}

#[derive(Debug, Clone)]
pub enum OptimizationMode {
    Auto,        // Automatic optimization decisions
    Aggressive,  // Prefer binary deployment
    Conservative,// Prefer SSH with selective binary optimization
    Off,         // SSH only
}

#[derive(Debug, Clone)]
pub struct DeploymentResult {
    pub success: bool,
    pub binary_deployments: Vec<BinaryDeploymentResult>,
    pub ssh_deployments: Vec<SshDeploymentResult>,
    pub total_duration: Duration,
    pub performance_gain: Option<f32>,
    pub errors: Vec<DeploymentError>,
}

#[derive(Debug, Clone)]
pub struct CapabilityReport {
    pub rust_status: ComponentStatus,
    pub zig_status: ComponentStatus,
    pub zigbuild_status: ComponentStatus,
    pub available_targets: Vec<String>,
    pub recommendations: Vec<SetupRecommendation>,
    pub readiness_level: ReadinessLevel,
}

#[derive(Debug, Clone)]
pub enum ComponentStatus {
    Available { version: String },
    Missing,
    Outdated { current: String, recommended: String },
    Error { message: String },
}

#[derive(Debug, Clone)]
pub enum ReadinessLevel {
    FullyReady,      // All components available, all targets supported
    MostlyReady,     // Some cross-compilation available
    BasicReady,      // Native compilation only
    NotReady,        // Missing essential components
}
```

## File and Package Structure

```
src/compilation/
├── mod.rs                     # Main compilation module
├── zero_infra.rs              # Zero-infrastructure compiler
├── zigbuild.rs                # Zig-based cross-compilation
├── capabilities.rs            # Capability detection
├── optimizer.rs               # Deployment optimization
├── cache.rs                   # Compilation caching
└── toolchain.rs               # Toolchain management

src/cli/
├── mod.rs                     # CLI module
├── commands.rs                # Command implementations
├── options.rs                 # Command-line options
├── output.rs                  # Output formatting and reporting
└── setup.rs                   # Initial setup and validation

src/deployment/
├── mod.rs                     # Deployment coordination
├── planner.rs                 # Deployment planning
├── executor.rs                # Mixed deployment execution
├── strategies.rs              # Deployment strategies
└── fallback.rs                # SSH fallback handling

src/bin/
├── rustle-deploy.rs           # Main binary entry point
└── capability-check.rs        # Capability checking utility

tests/compilation/
├── zero_infra_tests.rs        # Zero-infrastructure compilation tests
├── zigbuild_tests.rs          # Zig compilation tests
├── capabilities_tests.rs      # Capability detection tests
├── optimizer_tests.rs         # Optimization tests
└── integration_tests.rs       # End-to-end compilation tests

tests/fixtures/
├── toolchains/                # Mock toolchain configurations
├── playbooks/                 # Test playbooks for various scenarios
├── inventories/               # Multi-platform inventory files
└── expected_outputs/          # Expected compilation results
```

## Implementation Details

### Phase 1: Capability Detection and Basic Infrastructure

1. **Toolchain Detection**
   - Implement Rust toolchain discovery and validation
   - Add Zig installation detection with version checking
   - Create cargo-zigbuild availability checking
   - Build capability matrix for target platform support

2. **Target Specification System**
   - Define supported target triples and their requirements
   - Map inventory hosts to target architectures
   - Create target compatibility checking logic
   - Implement target grouping for batch compilation

3. **Basic CLI Interface**
   - Create command-line argument parsing matching ansible-playbook
   - Implement capability checking command (`--check-capabilities`)
   - Add basic deployment options and configuration
   - Set up logging and output formatting

### Phase 2: Zig-Based Cross-Compilation

1. **ZigBuild Integration**
   - Implement cargo-zigbuild wrapper with proper error handling
   - Add target-specific compilation configuration
   - Create compilation result validation and processing
   - Implement compilation caching for identical templates

2. **Template Generation for Cross-Compilation**
   - Extend template generator to work with zigbuild
   - Add platform-specific runtime configuration
   - Implement static linking and dependency bundling
   - Create binary size optimization for deployment

3. **Error Handling and Fallback**
   - Implement graceful compilation failure handling
   - Add automatic fallback to SSH deployment
   - Create detailed error reporting and suggestions
   - Implement retry logic for transient failures

### Phase 3: Deployment Optimization

1. **Performance Analysis**
   - Implement execution plan analysis for binary compatibility
   - Create performance prediction models
   - Add cost-benefit analysis for compilation vs SSH
   - Implement target-specific optimization recommendations

2. **Mixed Deployment Strategies**
   - Create hybrid deployment plans with binary + SSH
   - Implement parallel execution of different deployment methods
   - Add progress tracking and status reporting
   - Create deployment result aggregation and analysis

3. **Caching and Reuse**
   - Implement binary caching based on execution plan hashes
   - Add cache invalidation and cleanup logic
   - Create cache sharing across similar deployments
   - Implement incremental compilation for plan changes

### Phase 4: User Experience and Polish

1. **Setup Assistant**
   - Create interactive setup wizard for first-time users
   - Implement automatic dependency installation where possible
   - Add setup validation and troubleshooting guidance
   - Create configuration file management

2. **Performance Reporting**
   - Implement detailed performance metrics collection
   - Add deployment time comparison with baseline SSH
   - Create optimization recommendation engine
   - Implement performance trend tracking

3. **Advanced Features**
   - Add support for custom target specifications
   - Implement advanced caching strategies
   - Create plugin system for custom compilation steps
   - Add integration with CI/CD pipelines

### Key Algorithms

**Capability Detection Algorithm**:
```rust
impl ToolchainDetector {
    pub async fn detect_full_capabilities() -> Result<CompilationCapabilities, DetectionError> {
        let mut capabilities = CompilationCapabilities::default();
        
        // Check Rust installation
        if let Ok(rust) = self.check_rust_installation() {
            capabilities.rust_version = Some(rust.version);
            capabilities.native_target = env::consts::TARGET.to_string();
            
            // Check for basic cross-compilation targets
            capabilities.available_targets.insert(capabilities.native_target.clone());
            
            // Test common cross-compilation targets
            for target in COMMON_TARGETS {
                if self.test_target_compilation(target).await.is_ok() {
                    capabilities.available_targets.insert(target.to_string());
                }
            }
        }
        
        // Check Zig installation
        if let Ok(Some(zig)) = self.check_zig_installation() {
            capabilities.zig_available = true;
            
            // Check cargo-zigbuild
            if self.check_zigbuild_installation().is_ok() {
                capabilities.zigbuild_available = true;
                
                // Add all Zig-supported targets
                capabilities.available_targets.extend(ZIG_SUPPORTED_TARGETS.iter().map(|s| s.to_string()));
            }
        }
        
        // Determine capability level
        capabilities.capability_level = match (capabilities.zig_available, capabilities.zigbuild_available) {
            (true, true) => CapabilityLevel::Full,
            (false, false) if capabilities.available_targets.len() > 1 => CapabilityLevel::Limited,
            (false, false) => CapabilityLevel::Minimal,
            _ => CapabilityLevel::Limited,
        };
        
        Ok(capabilities)
    }
}
```

**Deployment Optimization Algorithm**:
```rust
impl DeploymentOptimizer {
    pub async fn create_optimal_deployment_plan(
        &self,
        execution_plan: &RustlePlanOutput,
        capabilities: &CompilationCapabilities,
    ) -> Result<DeploymentPlan, OptimizationError> {
        let mut plan = DeploymentPlan::new();
        
        // Group hosts by target architecture
        let target_groups = self.group_hosts_by_target(&execution_plan.inventory).await?;
        
        for (target_triple, hosts) in target_groups {
            let target_spec = TargetSpecification::from_triple(&target_triple)?;
            
            // Check if we can compile for this target
            let can_compile = capabilities.available_targets.contains(&target_triple);
            
            if can_compile {
                // Analyze tasks for binary compatibility
                let compatible_tasks = self.binary_analyzer.analyze_compatibility(
                    &execution_plan.tasks_for_hosts(&hosts)
                )?;
                
                // Decide based on cost-benefit analysis
                let decision = self.should_use_binary_deployment(&compatible_tasks, &target_spec);
                
                match decision {
                    BinaryDeploymentDecision::Recommended { .. } => {
                        // Generate template and compile binary
                        let template = self.generate_template(&compatible_tasks, &target_spec).await?;
                        let binary = self.compile_with_zigbuild(&template, &target_spec).await?;
                        
                        plan.binary_deployments.push(BinaryDeployment {
                            binary,
                            target_hosts: hosts.clone(),
                            deployment_method: BinaryDeploymentMethod::UploadAndExecute,
                        });
                        
                        // Add remaining tasks as SSH deployment
                        let remaining_tasks = execution_plan.tasks_excluding(&compatible_tasks);
                        if !remaining_tasks.is_empty() {
                            plan.ssh_deployments.push(SshDeployment {
                                execution_plan: ExecutionPlan::from_tasks(remaining_tasks),
                                target_hosts: hosts,
                                fallback_reason: FallbackReason::ModuleIncompatibility,
                            });
                        }
                    },
                    _ => {
                        // Use SSH deployment for all tasks
                        plan.ssh_deployments.push(SshDeployment {
                            execution_plan: ExecutionPlan::from_hosts_tasks(&hosts),
                            target_hosts: hosts,
                            fallback_reason: FallbackReason::ModuleIncompatibility,
                        });
                    }
                }
            } else {
                // SSH fallback for unsupported targets
                plan.ssh_deployments.push(SshDeployment {
                    execution_plan: ExecutionPlan::from_hosts_tasks(&hosts),
                    target_hosts: hosts,
                    fallback_reason: FallbackReason::UnsupportedTarget,
                });
            }
        }
        
        // Calculate overall performance metrics
        plan.estimated_performance_gain = self.estimate_performance_gain(
            plan.binary_deployments.len(),
            plan.ssh_deployments.len(),
            execution_plan.total_hosts(),
        );
        
        Ok(plan)
    }
}
```

## Testing Strategy

### Unit Tests
- **Capability Detection**: Test toolchain detection on various system configurations
- **ZigBuild Integration**: Test cross-compilation for all supported targets
- **Optimization Logic**: Test deployment strategy selection algorithms
- **Error Handling**: Test graceful degradation and fallback scenarios

### Integration Tests
- **End-to-End Deployment**: Test complete deployment pipelines with mixed strategies
- **Multi-Platform**: Test deployment to heterogeneous target environments
- **Performance Validation**: Verify performance gains against SSH-only deployment
- **Compatibility**: Test with real Ansible playbooks and inventories

### Test Infrastructure
```
tests/fixtures/zero_infra/
├── toolchain_configs/
│   ├── rust_only.json         # Rust without Zig configuration
│   ├── zig_available.json     # Zig + cargo-zigbuild available
│   └── minimal.json           # Minimal Rust installation
├── test_inventories/
│   ├── mixed_platforms.yml    # Linux, macOS, Windows hosts
│   ├── arm_cluster.yml        # ARM64 target cluster
│   └── single_platform.yml    # Homogeneous environment
├── sample_playbooks/
│   ├── cross_platform.yml     # Multi-OS deployment
│   ├── high_optimization.yml  # High binary compatibility
│   └── ssh_fallback.yml       # Low binary compatibility
└── expected_results/
    ├── deployment_plans/       # Expected optimization decisions
    ├── compilation_outputs/    # Expected binary artifacts
    └── performance_metrics/    # Expected performance improvements
```

## Edge Cases & Error Handling

### Cross-Compilation Edge Cases
- Zig version incompatibility with cargo-zigbuild
- Target architecture detection failures from inventory
- Compilation failures due to platform-specific dependencies
- Binary size exceeding deployment limits
- Cross-compilation taking longer than SSH deployment

### Capability Detection Edge Cases
- Partial Rust installations missing components
- Zig installed but not in PATH
- cargo-zigbuild installed but broken
- Network failures during toolchain validation
- Permission issues accessing development tools

### Deployment Edge Cases
- Mixed success/failure across different targets
- Binary deployment succeeding but execution failing
- SSH fallback failing after binary compilation attempt
- Inventory changes during deployment execution
- Host connectivity issues during target detection

### Recovery Strategies
- Automatic fallback to SSH when binary compilation fails
- Graceful degradation when some targets unsupported
- Caching of successful compilations for retry scenarios
- Clear error reporting with actionable recommendations
- Rollback capability for partial deployment failures

## Dependencies

### External Crates
```toml
[dependencies]
# Cross-compilation support
cargo-zigbuild = "0.17"        # Zig-based cross-compilation
which = "4.4"                  # Executable discovery
target-lexicon = "0.12"        # Target triple parsing

# Process execution
tokio-process = "0.2"          # Async process execution
command-group = "2.1"          # Process group management

# System information
sysinfo = "0.29"               # System and process information
dirs = "5.0"                   # Standard directories

# CLI and user interface
clap = { version = "4", features = ["derive"] }
console = "0.15"               # Terminal utilities
indicatif = "0.17"             # Progress bars and spinners

# Configuration and caching
dirs = "5.0"                   # User directories
tempfile = "3"                 # Temporary file management
```

### System Dependencies
- **Rust toolchain**: Required for all compilation
- **Zig**: Optional but recommended for full cross-compilation
- **cargo-zigbuild**: Optional plugin for Zig integration
- **SSH client**: Required for fallback deployments

## Configuration

### Zero-Infrastructure Configuration
```toml
[compilation]
# Capability detection
auto_detect_capabilities = true
cache_capability_results = true
capability_cache_duration_hours = 24

# Cross-compilation preferences
prefer_zig_when_available = true
parallel_compilation = true
max_compilation_jobs = 4

# Optimization settings
optimization_threshold = 0.3    # Minimum benefit for binary deployment
compilation_timeout_secs = 300
binary_cache_size_mb = 1024

# Fallback behavior
auto_fallback_on_failure = true
fallback_timeout_secs = 30
preserve_failed_artifacts = false

[deployment]
# Strategy selection
default_optimization_mode = "auto"
binary_deployment_preference = 0.7  # Bias toward binary when close
ssh_deployment_preference = 0.3

# Performance tuning
parallel_deployments = true
max_deployment_threads = 8
deployment_timeout_secs = 1800

[toolchain]
# Tool discovery
rust_discovery_paths = ["/usr/local/bin", "~/.cargo/bin"]
zig_discovery_paths = ["/usr/local/bin", "~/zig"]
auto_install_missing = false

# Version requirements
minimum_rust_version = "1.70.0"
recommended_zig_version = "0.11.0"
```

### Environment Variables
- `RUSTLE_ZIG_PATH`: Override Zig executable path
- `RUSTLE_CARGO_ZIGBUILD`: Override cargo-zigbuild path
- `RUSTLE_OPTIMIZATION_MODE`: Override optimization mode (auto, aggressive, conservative, off)
- `RUSTLE_CACHE_DIR`: Override compilation cache directory
- `RUSTLE_MAX_COMPILATION_JOBS`: Override parallel compilation limit

## Documentation

### Zero-Infrastructure Cross-Compilation Guide
```rust
/// Perform zero-infrastructure cross-compilation for multiple targets
/// 
/// This function automatically detects available toolchains and compiles
/// binaries for all compatible target architectures without requiring
/// Docker or cloud services.
/// 
/// # Arguments
/// * `template` - Generated binary template from execution plan
/// * `inventory` - Parsed inventory with target host information
/// 
/// # Returns
/// * `Ok(DeploymentPlan)` - Optimized deployment plan with binary + SSH strategies
/// * `Err(CompilationError)` - Compilation setup or execution failure
/// 
/// # Examples
/// ```rust
/// let compiler = ZeroInfraCompiler::detect_capabilities();
/// let plan = compiler.compile_or_fallback(&template, &inventory).await?;
/// 
/// for deployment in &plan.binary_deployments {
///     println!("Binary deployment: {} hosts", deployment.target_hosts.len());
/// }
/// 
/// for deployment in &plan.ssh_deployments {
///     println!("SSH fallback: {} hosts ({})", 
///              deployment.target_hosts.len(),
///              deployment.fallback_reason);
/// }
/// ```
```

### CLI Usage Examples
```bash
# Drop-in Ansible replacement
rustle-deploy playbook.yml -i inventory.yml

# Check cross-compilation capabilities
rustle-deploy --check-capabilities

# Force binary optimization
rustle-deploy playbook.yml -i inventory.yml --binary-only

# Conservative SSH-first approach
rustle-deploy playbook.yml -i inventory.yml --optimization=conservative

# Install recommended dependencies
rustle-deploy --setup

# Dry run with optimization analysis
rustle-deploy playbook.yml -i inventory.yml --dry-run --verbose
```

## Integration Points

### CLI Integration
```rust
impl RustleDeployCli {
    pub async fn execute_deployment(
        &self,
        playbook: &Path,
        inventory: &Path,
        options: DeployOptions,
    ) -> Result<DeploymentResult, DeploymentError> {
        // Parse execution plan
        let execution_plan = self.parse_execution_plan(playbook, inventory).await?;
        
        // Create deployment plan based on capabilities
        let deployment_plan = self.compiler.compile_or_fallback(
            &execution_plan.template,
            &execution_plan.inventory,
        ).await?;
        
        // Execute mixed deployment strategy
        let result = self.execute_deployment_plan(deployment_plan).await?;
        
        // Report results
        self.print_deployment_summary(&result);
        
        Ok(result)
    }
}
```

### Template Generation Integration
```rust
impl BinaryTemplateGenerator {
    pub fn generate_for_zero_infra(
        &self,
        execution_plan: &RustlePlanOutput,
        target: &TargetSpecification,
    ) -> Result<GeneratedTemplate, TemplateError> {
        let mut template = self.generate_binary_template(execution_plan, target)?;
        
        // Optimize for Zig cross-compilation
        if target.requires_zig {
            template = self.optimize_for_zigbuild(template)?;
        }
        
        // Add zero-infrastructure runtime
        template.add_zero_infra_runtime()?;
        
        Ok(template)
    }
}
```

The zero-infrastructure cross-compilation system provides the foundation for rustle-deploy to be a true drop-in Ansible replacement with significant performance gains while maintaining simplicity and ease of adoption.