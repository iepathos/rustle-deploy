# Spec 090: Cross-Compilation Infrastructure

## Feature Summary

Implement a comprehensive cross-compilation infrastructure that manages Rust toolchains, target platforms, and compilation environments to enable rustle-deploy to compile binaries for multiple architectures. This system provides automated toolchain management, Docker-based compilation environments, and intelligent target detection.

**Problem it solves**: Binary deployment requires compiling for diverse target architectures (x86_64, ARM64, different operating systems) from a single controller host. The current system lacks the infrastructure to manage cross-compilation toolchains and handle platform-specific compilation requirements.

**High-level approach**: Create a robust cross-compilation system that automatically manages Rust toolchains, provides Docker-based compilation environments for isolation, detects target architectures from inventory, and optimizes compilation for each platform while handling cross-compilation complexity transparently.

## Goals & Requirements

### Functional Requirements
- Automatic Rust toolchain installation and management
- Cross-compilation support for major platforms (Linux x86_64/ARM64, macOS, Windows)
- Docker-based compilation environments for isolation and consistency
- Target architecture detection from inventory and system information
- Platform-specific optimization and feature detection
- Compilation caching and incremental builds across targets
- Error handling and fallback strategies for compilation failures
- Integration with existing binary template generation
- Support for custom target specifications
- Automated dependency resolution for cross-compilation

### Non-functional Requirements
- **Performance**: Compile binaries for 10+ targets in parallel in <5 minutes
- **Reliability**: 99%+ compilation success rate for supported targets
- **Isolation**: Docker-based compilation prevents environment conflicts
- **Efficiency**: Intelligent caching reduces rebuild time by 90%+
- **Flexibility**: Support for custom targets and compilation flags

### Success Criteria
- Successfully cross-compile for all major target platforms
- Automatic toolchain management requires no manual intervention
- Docker-based compilation provides consistent results across environments
- Target detection accurately identifies architecture requirements
- Compilation caching significantly improves incremental build performance

## API/Interface Design

### Core Cross-Compilation System

```rust
/// Cross-compilation manager that handles toolchains and target compilation
pub struct CrossCompilationManager {
    config: CrossCompilationConfig,
    toolchain_manager: ToolchainManager,
    docker_manager: DockerCompilationManager,
    target_detector: TargetDetector,
    cache: CompilationCache,
}

impl CrossCompilationManager {
    pub fn new(config: CrossCompilationConfig) -> Self;
    
    pub async fn setup_environment(&self) -> Result<(), CrossCompilationError>;
    
    pub async fn detect_targets_from_inventory(
        &self,
        inventory: &ParsedInventory,
    ) -> Result<Vec<TargetSpecification>, TargetDetectionError>;
    
    pub async fn install_required_toolchains(
        &self,
        targets: &[TargetSpecification],
    ) -> Result<(), ToolchainError>;
    
    pub async fn cross_compile_binary(
        &self,
        template: &GeneratedTemplate,
        target: &TargetSpecification,
        optimization: CompilationOptimization,
    ) -> Result<CompiledBinary, CompilationError>;
    
    pub async fn cross_compile_parallel(
        &self,
        template: &GeneratedTemplate,
        targets: &[TargetSpecification],
    ) -> Result<Vec<CompiledBinary>, CompilationError>;
    
    pub fn get_supported_targets(&self) -> &[TargetSpecification];
    
    pub async fn validate_target_compatibility(
        &self,
        target: &TargetSpecification,
    ) -> Result<CompatibilityReport, ValidationError>;
}

/// Target platform specification
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TargetSpecification {
    pub triple: String,
    pub platform: Platform,
    pub architecture: Architecture,
    pub os_family: OsFamily,
    pub libc: Option<LibcType>,
    pub features: Vec<String>,
    pub rustflags: Vec<String>,
    pub docker_image: Option<String>,
    pub linker: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Architecture {
    X86_64,
    Aarch64,
    Armv7,
    I686,
    Mips,
    Riscv64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OsFamily {
    Linux,
    MacOS,
    Windows,
    FreeBSD,
    NetBSD,
    OpenBSD,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LibcType {
    Glibc,
    Musl,
    Msvcrt,
}

#[derive(Debug, Clone)]
pub struct CompilationOptimization {
    pub level: OptimizationLevel,
    pub lto: bool,
    pub codegen_units: Option<u16>,
    pub target_cpu: Option<String>,
    pub target_features: Vec<String>,
    pub strip: bool,
    pub panic_strategy: PanicStrategy,
}

#[derive(Debug, Clone)]
pub enum PanicStrategy {
    Unwind,
    Abort,
}

#[derive(Debug, Clone)]
pub struct CompiledBinary {
    pub target: TargetSpecification,
    pub binary_data: Vec<u8>,
    pub size: u64,
    pub checksum: String,
    pub compilation_time: Duration,
    pub metadata: CompilationMetadata,
}

#[derive(Debug, Clone)]
pub struct CompilationMetadata {
    pub rustc_version: String,
    pub cargo_version: String,
    pub compilation_flags: Vec<String>,
    pub dependencies: Vec<String>,
    pub features_used: Vec<String>,
}
```

### Toolchain Management

```rust
/// Manages Rust toolchains for cross-compilation
pub struct ToolchainManager {
    rustup_path: PathBuf,
    toolchain_dir: PathBuf,
    default_toolchain: String,
}

impl ToolchainManager {
    pub fn new(config: &ToolchainConfig) -> Self;
    
    pub async fn install_toolchain(&self, target: &str) -> Result<(), ToolchainError>;
    
    pub async fn list_installed_toolchains(&self) -> Result<Vec<String>, ToolchainError>;
    
    pub async fn add_target(
        &self,
        toolchain: &str,
        target: &str,
    ) -> Result<(), ToolchainError>;
    
    pub async fn get_target_availability(
        &self,
        target: &str,
    ) -> Result<TargetAvailability, ToolchainError>;
    
    pub async fn update_toolchains(&self) -> Result<(), ToolchainError>;
    
    pub fn get_rustc_path(&self, target: &str) -> Result<PathBuf, ToolchainError>;
    
    pub fn get_cargo_path(&self, target: &str) -> Result<PathBuf, ToolchainError>;
}

#[derive(Debug, Clone)]
pub enum TargetAvailability {
    Available,
    RequiresInstallation,
    NotSupported,
}

pub struct ToolchainConfig {
    pub auto_install: bool,
    pub update_interval: Duration,
    pub default_toolchain: String,
    pub custom_toolchains: HashMap<String, String>,
}
```

### Docker-Based Compilation

```rust
/// Manages Docker-based cross-compilation environments
pub struct DockerCompilationManager {
    docker_client: Docker,
    image_registry: ImageRegistry,
    build_cache: BuildCache,
}

impl DockerCompilationManager {
    pub fn new() -> Result<Self, DockerError>;
    
    pub async fn setup_compilation_environment(
        &self,
        target: &TargetSpecification,
    ) -> Result<CompilationEnvironment, DockerError>;
    
    pub async fn compile_in_docker(
        &self,
        template: &GeneratedTemplate,
        target: &TargetSpecification,
        environment: &CompilationEnvironment,
    ) -> Result<CompiledBinary, CompilationError>;
    
    pub async fn pull_required_images(
        &self,
        targets: &[TargetSpecification],
    ) -> Result<(), DockerError>;
    
    pub async fn cleanup_build_containers(&self) -> Result<(), DockerError>;
    
    pub fn get_image_for_target(
        &self,
        target: &TargetSpecification,
    ) -> Result<String, DockerError>;
}

#[derive(Debug, Clone)]
pub struct CompilationEnvironment {
    pub container_id: String,
    pub image: String,
    pub working_dir: PathBuf,
    pub environment_vars: HashMap<String, String>,
    pub volume_mounts: Vec<VolumeMount>,
}

#[derive(Debug, Clone)]
pub struct VolumeMount {
    pub host_path: PathBuf,
    pub container_path: PathBuf,
    pub read_only: bool,
}

pub struct ImageRegistry {
    images: HashMap<String, DockerImage>,
}

#[derive(Debug, Clone)]
pub struct DockerImage {
    pub name: String,
    pub tag: String,
    pub supported_targets: Vec<String>,
    pub toolchain_version: String,
}
```

### Target Detection System

```rust
/// Detects target architectures from inventory and system information
pub struct TargetDetector {
    platform_detectors: HashMap<String, Box<dyn PlatformDetector>>,
    inventory_analyzer: InventoryAnalyzer,
}

impl TargetDetector {
    pub fn new() -> Self;
    
    pub async fn detect_from_inventory(
        &self,
        inventory: &ParsedInventory,
    ) -> Result<Vec<TargetSpecification>, TargetDetectionError>;
    
    pub async fn detect_from_host(
        &self,
        host: &HostConfig,
    ) -> Result<TargetSpecification, TargetDetectionError>;
    
    pub fn detect_from_facts(
        &self,
        facts: &HashMap<String, Value>,
    ) -> Result<TargetSpecification, TargetDetectionError>;
    
    pub fn validate_target(
        &self,
        target: &TargetSpecification,
    ) -> Result<bool, ValidationError>;
    
    pub async fn probe_host_architecture(
        &self,
        host: &str,
        connection_config: &ConnectionConfig,
    ) -> Result<ArchitectureInfo, ProbeError>;
}

#[async_trait]
pub trait PlatformDetector: Send + Sync {
    async fn detect_architecture(
        &self,
        host: &str,
        connection: &Connection,
    ) -> Result<ArchitectureInfo, DetectionError>;
    
    fn supported_platforms(&self) -> &[Platform];
}

#[derive(Debug, Clone)]
pub struct ArchitectureInfo {
    pub architecture: Architecture,
    pub platform: Platform,
    pub os_family: OsFamily,
    pub os_version: String,
    pub libc_type: Option<LibcType>,
    pub cpu_features: Vec<String>,
    pub endianness: Endianness,
}

#[derive(Debug, Clone)]
pub enum Endianness {
    Little,
    Big,
}

pub struct LinuxDetector;
pub struct MacOSDetector;
pub struct WindowsDetector;
```

## File and Package Structure

```
src/cross_compile/
├── mod.rs                     # Cross-compilation module
├── manager.rs                 # Main cross-compilation manager
├── toolchain.rs               # Toolchain management
├── docker.rs                  # Docker-based compilation
├── target_detector.rs         # Target architecture detection
├── cache.rs                   # Compilation caching
├── optimization.rs            # Compilation optimization
└── platform/                  # Platform-specific detectors
    ├── linux.rs              # Linux platform detection
    ├── macos.rs               # macOS platform detection
    └── windows.rs             # Windows platform detection

src/compiler/
├── cross_compile.rs           # Cross-compilation integration (update existing)
└── targets.rs                 # Target management (update existing)

docker/
├── cross-compile/             # Docker images for cross-compilation
│   ├── linux-x86_64.dockerfile
│   ├── linux-aarch64.dockerfile
│   ├── macos.dockerfile
│   └── windows.dockerfile
└── scripts/
    ├── setup-toolchain.sh     # Toolchain setup scripts
    └── compile-binary.sh      # Compilation scripts

tests/cross_compile/
├── manager_tests.rs           # Cross-compilation manager tests
├── toolchain_tests.rs         # Toolchain management tests
├── docker_tests.rs            # Docker compilation tests
├── target_detection_tests.rs  # Target detection tests
└── integration_tests.rs       # End-to-end cross-compilation tests
```

## Implementation Details

### Phase 1: Basic Cross-Compilation
1. Implement toolchain manager with rustup integration
2. Create basic target detection from inventory
3. Add simple cross-compilation without Docker
4. Integrate with existing binary compilation

### Phase 2: Docker Integration
1. Implement Docker-based compilation environments
2. Create Docker images for major target platforms
3. Add container management and cleanup
4. Integrate Docker compilation with toolchain manager

### Phase 3: Advanced Target Detection
1. Implement platform-specific detectors
2. Add SSH-based architecture probing
3. Create inventory analysis for target detection
4. Add validation and compatibility checking

### Phase 4: Optimization and Caching
1. Implement compilation caching across targets
2. Add parallel compilation support
3. Create optimization strategies for different targets
4. Add performance monitoring and metrics

### Key Cross-Compilation Algorithms

**Target Detection from Inventory**:
```rust
impl TargetDetector {
    pub async fn detect_from_inventory(
        &self,
        inventory: &ParsedInventory,
    ) -> Result<Vec<TargetSpecification>, TargetDetectionError> {
        let mut targets = Vec::new();
        let mut target_groups = HashMap::new();
        
        // Analyze each host in inventory
        for (host_name, host_config) in &inventory.hosts {
            let target = if let Some(explicit_target) = &host_config.target_triple {
                // Use explicitly specified target
                TargetSpecification::from_triple(explicit_target)?
            } else {
                // Detect target from host connection
                self.detect_from_host(host_config).await?
            };
            
            // Group hosts by target architecture
            target_groups.entry(target.triple.clone())
                .or_insert_with(|| (target.clone(), Vec::new()))
                .1
                .push(host_name.clone());
        }
        
        // Convert to target specifications
        for (target, hosts) in target_groups.into_values() {
            targets.push(target);
        }
        
        Ok(targets)
    }
    
    pub async fn detect_from_host(
        &self,
        host_config: &HostConfig,
    ) -> Result<TargetSpecification, TargetDetectionError> {
        // Try to detect from existing facts
        if let Some(facts) = &host_config.cached_facts {
            if let Ok(target) = self.detect_from_facts(facts) {
                return Ok(target);
            }
        }
        
        // Probe host architecture via SSH
        let connection_config = &host_config.connection;
        let arch_info = self.probe_host_architecture(
            &host_config.address,
            connection_config,
        ).await?;
        
        Ok(TargetSpecification::from_architecture_info(&arch_info))
    }
}
```

**Docker-Based Compilation**:
```rust
impl DockerCompilationManager {
    pub async fn compile_in_docker(
        &self,
        template: &GeneratedTemplate,
        target: &TargetSpecification,
        environment: &CompilationEnvironment,
    ) -> Result<CompiledBinary, CompilationError> {
        // Create temporary directory for compilation
        let temp_dir = tempfile::tempdir()?;
        let project_path = temp_dir.path();
        
        // Write template files to project directory
        for (file_path, content) in &template.source_files {
            let full_path = project_path.join(file_path);
            if let Some(parent) = full_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::write(full_path, content).await?;
        }
        
        // Write Cargo.toml
        tokio::fs::write(project_path.join("Cargo.toml"), &template.cargo_toml).await?;
        
        // Set up Docker container
        let container = self.docker_client.create_container(
            Some(CreateContainerOptions {
                name: format!("rustle-compile-{}", uuid::Uuid::new_v4()),
            }),
            Config {
                image: Some(environment.image.clone()),
                working_dir: Some("/workspace".to_string()),
                env: Some(
                    environment.environment_vars
                        .iter()
                        .map(|(k, v)| format!("{}={}", k, v))
                        .collect(),
                ),
                host_config: Some(HostConfig {
                    binds: Some(vec![
                        format!("{}:/workspace:rw", project_path.display()),
                    ]),
                    ..Default::default()
                }),
                ..Default::default()
            },
        ).await?;
        
        // Start container
        self.docker_client.start_container(&container.id, None::<StartContainerOptions<String>>).await?;
        
        // Execute compilation
        let compilation_cmd = format!(
            "cargo build --release --target {} {}",
            target.triple,
            template.compilation_flags.join(" ")
        );
        
        let exec = self.docker_client.create_exec(
            &container.id,
            CreateExecOptions {
                cmd: Some(vec!["sh", "-c", &compilation_cmd]),
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                ..Default::default()
            },
        ).await?;
        
        let output = self.docker_client.start_exec(&exec.id, None).await?;
        
        // Check compilation success
        let binary_path = format!("target/{}/release/rustle-runner", target.triple);
        let binary_data = tokio::fs::read(project_path.join(&binary_path)).await?;
        
        // Cleanup container
        self.docker_client.remove_container(&container.id, None::<RemoveContainerOptions>).await.ok();
        
        Ok(CompiledBinary {
            target: target.clone(),
            binary_data,
            size: binary_data.len() as u64,
            checksum: calculate_checksum(&binary_data),
            compilation_time: start_time.elapsed(),
            metadata: CompilationMetadata {
                rustc_version: "1.70.0".to_string(), // Get from container
                cargo_version: "1.70.0".to_string(),
                compilation_flags: template.compilation_flags.clone(),
                dependencies: vec![], // Extract from Cargo.lock
                features_used: vec![],
            },
        })
    }
}
```

**Parallel Cross-Compilation**:
```rust
impl CrossCompilationManager {
    pub async fn cross_compile_parallel(
        &self,
        template: &GeneratedTemplate,
        targets: &[TargetSpecification],
    ) -> Result<Vec<CompiledBinary>, CompilationError> {
        let semaphore = Arc::new(Semaphore::new(self.config.max_parallel_jobs));
        let mut handles = Vec::new();
        
        for target in targets {
            let template = template.clone();
            let target = target.clone();
            let manager = self.clone();
            let permit = semaphore.clone().acquire_owned().await?;
            
            let handle = tokio::spawn(async move {
                let _permit = permit;
                manager.cross_compile_binary(
                    &template,
                    &target,
                    CompilationOptimization::default(),
                ).await
            });
            
            handles.push(handle);
        }
        
        let mut results = Vec::new();
        for handle in handles {
            match handle.await? {
                Ok(binary) => results.push(binary),
                Err(e) => return Err(e),
            }
        }
        
        Ok(results)
    }
}
```

## Testing Strategy

### Unit Tests
- **Toolchain Management**: Test toolchain installation and management
- **Target Detection**: Test architecture detection from various sources
- **Docker Integration**: Test Docker container management and compilation
- **Compilation**: Test cross-compilation for different targets

### Integration Tests
- **End-to-End**: Test complete cross-compilation pipeline
- **Multi-Target**: Test parallel compilation for multiple targets
- **Real Hosts**: Test target detection with real inventory
- **Docker Environments**: Test compilation in different Docker images

### Test Infrastructure
```
tests/fixtures/cross_compile/
├── targets/
│   ├── linux_x86_64.json     # Target specification
│   ├── linux_aarch64.json    # ARM64 target
│   └── macos_x86_64.json     # macOS target
├── inventories/
│   ├── multi_arch.yml        # Multi-architecture inventory
│   └── single_arch.yml       # Single architecture
├── docker_images/
│   ├── test_images.json      # Test Docker image registry
│   └── image_configs/        # Docker image configurations
└── templates/
    ├── simple_binary/        # Simple binary template
    └── complex_binary/       # Complex multi-module template
```

## Edge Cases & Error Handling

### Cross-Compilation Edge Cases
- Target architectures not supported by Rust
- Missing system dependencies for cross-compilation
- Docker daemon unavailable or misconfigured
- Network failures during toolchain installation
- Insufficient disk space for compilation

### Target Detection Edge Cases
- Hosts with unknown or custom architectures
- SSH connections that fail during probing
- Inventory with inconsistent architecture information
- Hosts behind firewalls or NAT
- Mixed architectures in single host group

### Docker Compilation Edge Cases
- Docker images that fail to pull or start
- Compilation that exceeds memory or time limits
- Container filesystem issues
- Missing dependencies in Docker images
- Volume mount permissions issues

### Recovery Strategies
- Fallback to native compilation when Docker unavailable
- Retry with different Docker images on failure
- Graceful degradation for unsupported targets
- Alternative target detection methods
- Incremental compilation recovery after failures

## Dependencies

### External Crates
```toml
[dependencies]
# Docker integration
bollard = "0.14"               # Docker client
docker-api = "0.13"            # Alternative Docker client

# Process and system interaction
tokio-process = "0.2"          # Async process execution
sysinfo = "0.29"               # System information
nix = "0.27"                   # Unix system calls (Unix only)

# Architecture detection
target-lexicon = "0.12"        # Target triple parsing
platforms = "3.0"              # Platform detection

# Toolchain management
which = "4.4"                  # Executable discovery
dirs = "5.0"                   # Standard directories

# Compression and archives
tar = "0.4"                    # TAR archive handling
xz2 = "0.1"                    # XZ compression
```

### System Dependencies
- **Docker**: Required for isolated cross-compilation
- **rustup**: Rust toolchain management
- **git**: For toolchain and component installation
- **ssh**: For host architecture probing

## Configuration

### Cross-Compilation Configuration
```toml
[cross_compilation]
# Docker settings
use_docker = true
docker_registry = "docker.io"
cleanup_containers = true
container_timeout_secs = 1800

# Toolchain management
auto_install_toolchains = true
rustup_path = "rustup"
toolchain_update_interval_days = 7

# Compilation settings
max_parallel_jobs = 8
compilation_timeout_secs = 600
enable_incremental = true
cache_enabled = true

# Target support
supported_targets = [
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-gnu"
]

# Docker images
[cross_compilation.docker_images]
"x86_64-unknown-linux-gnu" = "rustembedded/cross:x86_64-unknown-linux-gnu"
"aarch64-unknown-linux-gnu" = "rustembedded/cross:aarch64-unknown-linux-gnu"
```

### Environment Variables
- `RUSTLE_DOCKER_REGISTRY`: Docker registry for cross-compilation images
- `RUSTLE_RUSTUP_PATH`: Path to rustup executable
- `RUSTLE_CROSS_COMPILE_TIMEOUT`: Compilation timeout in seconds
- `RUSTLE_TARGET_CACHE_DIR`: Target cache directory

## Documentation

### Cross-Compilation Guide
```rust
/// Cross-compile binary for multiple target architectures
/// 
/// # Arguments
/// * `template` - Generated binary template
/// * `targets` - Target architectures to compile for
/// 
/// # Returns
/// * `Ok(Vec<CompiledBinary>)` - Compiled binaries for each target
/// * `Err(CompilationError)` - Compilation failure
/// 
/// # Examples
/// ```rust
/// let manager = CrossCompilationManager::new(config);
/// let targets = manager.detect_targets_from_inventory(&inventory).await?;
/// let binaries = manager.cross_compile_parallel(&template, &targets).await?;
/// ```
```

### Usage Examples
```rust
// Basic cross-compilation
let manager = CrossCompilationManager::new(config);
let targets = vec![
    TargetSpecification::from_triple("x86_64-unknown-linux-gnu")?,
    TargetSpecification::from_triple("aarch64-unknown-linux-gnu")?,
];

let binaries = manager.cross_compile_parallel(&template, &targets).await?;

// Target detection from inventory
let detected_targets = manager.detect_targets_from_inventory(&inventory).await?;
let binaries = manager.cross_compile_parallel(&template, &detected_targets).await?;
```

## Integration Points

### Deployment Manager Integration
```rust
impl DeploymentManager {
    pub async fn compile_for_all_targets(
        &self,
        template: &GeneratedTemplate,
        inventory: &ParsedInventory,
    ) -> Result<Vec<CompiledBinary>, DeployError> {
        let targets = self.cross_compiler.detect_targets_from_inventory(inventory).await?;
        let binaries = self.cross_compiler.cross_compile_parallel(template, &targets).await?;
        Ok(binaries)
    }
}
```

The cross-compilation infrastructure integrates seamlessly with:
1. **Template Generation**: Compiles generated templates
2. **Target Detection**: Automatically determines required targets
3. **Binary Deployment**: Provides compiled binaries for deployment
4. **Caching System**: Leverages compilation cache for efficiency