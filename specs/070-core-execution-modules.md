# Spec 070: Core Execution Modules

## Feature Summary

Implement the core Ansible-compatible execution modules (command, package, debug, service, copy, template, etc.) as native Rust implementations that can be embedded into binary deployments. These modules provide the fundamental task execution capabilities needed for configuration management while maintaining full Ansible compatibility.

**Problem it solves**: Binary deployments need native Rust implementations of Ansible modules to execute tasks locally without network round-trips. The current codebase lacks these core execution modules, making binary deployment non-functional.

**High-level approach**: Create a comprehensive library of Rust-native execution modules that mirror Ansible module functionality, implement a unified module interface, and provide seamless embedding into binary deployments with optimal performance.

## Goals & Requirements

### Functional Requirements
- Implement core Ansible modules in native Rust
- Maintain 100% compatibility with Ansible module arguments and behavior
- Support all major module categories (system, commands, files, packages, services)
- Provide unified module execution interface
- Enable static linking and binary embedding
- Support cross-platform execution (Linux, macOS, Windows)
- Handle module result formatting and error reporting
- Support idempotent operations and change detection
- Implement module facts collection and variable updates
- Provide secure execution with privilege management

### Non-functional Requirements
- **Performance**: 10x faster execution compared to Python Ansible modules
- **Memory**: Minimal memory footprint when embedded in binaries
- **Compatibility**: 100% argument and behavior compatibility with Ansible
- **Security**: Safe execution with proper privilege isolation
- **Reliability**: 99.9%+ success rate for supported operations

### Success Criteria
- All modules from example JSON (debug, package, command, service) implemented
- Full Ansible argument compatibility maintained
- Binary embedding without bloat (total <5MB for core modules)
- Cross-platform support for major operating systems
- Comprehensive test suite with real-world scenarios

## API/Interface Design

### Core Module Interface

```rust
/// Unified interface for all execution modules
#[async_trait]
pub trait ExecutionModule: Send + Sync {
    /// Module name (e.g., "command", "package", "debug")
    fn name(&self) -> &'static str;
    
    /// Module version
    fn version(&self) -> &'static str;
    
    /// Supported platforms
    fn supported_platforms(&self) -> &[Platform];
    
    /// Execute the module with given arguments
    async fn execute(
        &self,
        args: &ModuleArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError>;
    
    /// Validate module arguments before execution
    fn validate_args(&self, args: &ModuleArgs) -> Result<(), ValidationError>;
    
    /// Check if module operation would make changes (dry-run)
    async fn check_mode(
        &self,
        args: &ModuleArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError>;
    
    /// Get module documentation
    fn documentation(&self) -> ModuleDocumentation;
}

/// Module execution arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleArgs {
    /// Direct module arguments
    pub args: HashMap<String, serde_json::Value>,
    /// Special parameters
    pub special: SpecialParameters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialParameters {
    pub become: Option<BecomeConfig>,
    pub when: Option<String>,
    pub changed_when: Option<String>,
    pub failed_when: Option<String>,
    pub check_mode: bool,
    pub diff: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BecomeConfig {
    pub method: String,  // sudo, su, runas, etc.
    pub user: String,
    pub password: Option<String>,
    pub flags: Vec<String>,
}

/// Module execution context
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub facts: HashMap<String, serde_json::Value>,
    pub variables: HashMap<String, serde_json::Value>,
    pub host_info: HostInfo,
    pub working_directory: PathBuf,
    pub environment: HashMap<String, String>,
    pub check_mode: bool,
    pub diff_mode: bool,
    pub verbosity: u8,
}

#[derive(Debug, Clone)]
pub struct HostInfo {
    pub hostname: String,
    pub platform: Platform,
    pub architecture: String,
    pub os_family: String,
    pub distribution: Option<String>,
    pub distribution_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Platform {
    Linux,
    MacOS,
    Windows,
    FreeBSD,
    OpenBSD,
    NetBSD,
}

/// Module execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleResult {
    pub changed: bool,
    pub failed: bool,
    pub msg: Option<String>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub rc: Option<i32>,
    pub results: HashMap<String, serde_json::Value>,
    pub diff: Option<Diff>,
    pub warnings: Vec<String>,
    pub ansible_facts: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diff {
    pub before: Option<String>,
    pub after: Option<String>,
    pub before_header: Option<String>,
    pub after_header: Option<String>,
}
```

### Core Module Implementations

#### 1. Debug Module
```rust
/// Debug module - displays messages and variables
pub struct DebugModule;

#[async_trait]
impl ExecutionModule for DebugModule {
    fn name(&self) -> &'static str { "debug" }
    fn version(&self) -> &'static str { "1.0.0" }
    fn supported_platforms(&self) -> &[Platform] { &[Platform::Linux, Platform::MacOS, Platform::Windows] }
    
    async fn execute(
        &self,
        args: &ModuleArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        let msg = args.args.get("msg")
            .and_then(|v| v.as_str())
            .unwrap_or("Hello world!");
        
        let var_name = args.args.get("var").and_then(|v| v.as_str());
        
        let output = if let Some(var) = var_name {
            if let Some(value) = context.variables.get(var) {
                format!("{}: {}", var, serde_json::to_string_pretty(value)?)
            } else {
                format!("{}: VARIABLE IS NOT DEFINED!", var)
            }
        } else {
            msg.to_string()
        };
        
        println!("{}", output);
        
        Ok(ModuleResult {
            changed: false,
            failed: false,
            msg: Some(output),
            stdout: None,
            stderr: None,
            rc: Some(0),
            results: HashMap::new(),
            diff: None,
            warnings: Vec::new(),
            ansible_facts: HashMap::new(),
        })
    }
    
    fn validate_args(&self, args: &ModuleArgs) -> Result<(), ValidationError> {
        // Debug module accepts any arguments
        Ok(())
    }
    
    async fn check_mode(
        &self,
        args: &ModuleArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        // Debug module has no side effects
        self.execute(args, context).await
    }
}
```

#### 2. Command Module
```rust
/// Command module - executes shell commands
pub struct CommandModule;

#[async_trait]
impl ExecutionModule for CommandModule {
    fn name(&self) -> &'static str { "command" }
    fn version(&self) -> &'static str { "1.0.0" }
    fn supported_platforms(&self) -> &[Platform] { &[Platform::Linux, Platform::MacOS, Platform::Windows] }
    
    async fn execute(
        &self,
        args: &ModuleArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        let command = self.extract_command(args)?;
        let chdir = args.args.get("chdir").and_then(|v| v.as_str());
        let creates = args.args.get("creates").and_then(|v| v.as_str());
        let removes = args.args.get("removes").and_then(|v| v.as_str());
        
        // Check creates/removes conditions
        if let Some(creates_path) = creates {
            if Path::new(creates_path).exists() {
                return Ok(ModuleResult {
                    changed: false,
                    failed: false,
                    msg: Some(format!("{} already exists", creates_path)),
                    stdout: None,
                    stderr: None,
                    rc: Some(0),
                    results: HashMap::new(),
                    diff: None,
                    warnings: Vec::new(),
                    ansible_facts: HashMap::new(),
                });
            }
        }
        
        if let Some(removes_path) = removes {
            if !Path::new(removes_path).exists() {
                return Ok(ModuleResult {
                    changed: false,
                    failed: false,
                    msg: Some(format!("{} does not exist", removes_path)),
                    stdout: None,
                    stderr: None,
                    rc: Some(0),
                    results: HashMap::new(),
                    diff: None,
                    warnings: Vec::new(),
                    ansible_facts: HashMap::new(),
                });
            }
        }
        
        if context.check_mode {
            return Ok(ModuleResult {
                changed: true,
                failed: false,
                msg: Some("Command would run".to_string()),
                stdout: None,
                stderr: None,
                rc: None,
                results: HashMap::new(),
                diff: None,
                warnings: Vec::new(),
                ansible_facts: HashMap::new(),
            });
        }
        
        let mut cmd = self.build_command(&command, context)?;
        
        if let Some(dir) = chdir {
            cmd.current_dir(dir);
        }
        
        let output = cmd.output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let rc = output.status.code().unwrap_or(-1);
        
        Ok(ModuleResult {
            changed: true,
            failed: !output.status.success(),
            msg: if output.status.success() { None } else { Some(stderr.clone()) },
            stdout: Some(stdout),
            stderr: Some(stderr),
            rc: Some(rc),
            results: HashMap::new(),
            diff: None,
            warnings: Vec::new(),
            ansible_facts: HashMap::new(),
        })
    }
    
    fn validate_args(&self, args: &ModuleArgs) -> Result<(), ValidationError> {
        if !args.args.contains_key("_raw_params") && !args.args.contains_key("cmd") {
            return Err(ValidationError::MissingRequiredArg("_raw_params or cmd".to_string()));
        }
        Ok(())
    }
}

impl CommandModule {
    fn extract_command(&self, args: &ModuleArgs) -> Result<Vec<String>, ModuleExecutionError> {
        if let Some(raw_params) = args.args.get("_raw_params") {
            if let Some(cmd_str) = raw_params.as_str() {
                // Split command respecting quotes
                return Ok(shell_words::split(cmd_str)?);
            }
        }
        
        if let Some(cmd) = args.args.get("cmd") {
            if let Some(cmd_str) = cmd.as_str() {
                return Ok(shell_words::split(cmd_str)?);
            }
            if let Some(cmd_array) = cmd.as_array() {
                return Ok(cmd_array.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect());
            }
        }
        
        Err(ModuleExecutionError::InvalidArgs("No command specified".to_string()))
    }
    
    fn build_command(
        &self,
        command: &[String],
        context: &ExecutionContext,
    ) -> Result<tokio::process::Command, ModuleExecutionError> {
        if command.is_empty() {
            return Err(ModuleExecutionError::InvalidArgs("Empty command".to_string()));
        }
        
        let mut cmd = tokio::process::Command::new(&command[0]);
        if command.len() > 1 {
            cmd.args(&command[1..]);
        }
        
        // Set environment variables
        for (key, value) in &context.environment {
            cmd.env(key, value);
        }
        
        Ok(cmd)
    }
}
```

#### 3. Package Module
```rust
/// Package module - manages system packages
pub struct PackageModule {
    package_managers: HashMap<Platform, Box<dyn PackageManager>>,
}

#[async_trait]
impl ExecutionModule for PackageModule {
    fn name(&self) -> &'static str { "package" }
    fn version(&self) -> &'static str { "1.0.0" }
    fn supported_platforms(&self) -> &[Platform] { &[Platform::Linux, Platform::MacOS, Platform::Windows] }
    
    async fn execute(
        &self,
        args: &ModuleArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        let name = args.args.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ModuleExecutionError::InvalidArgs("name is required".to_string()))?;
        
        let state = args.args.get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("present");
        
        let package_manager = self.package_managers.get(&context.host_info.platform)
            .ok_or_else(|| ModuleExecutionError::UnsupportedPlatform(context.host_info.platform.clone()))?;
        
        let current_state = package_manager.query_package(name).await?;
        
        let target_state = match state {
            "present" | "installed" | "latest" => PackageState::Present,
            "absent" | "removed" => PackageState::Absent,
            _ => return Err(ModuleExecutionError::InvalidArgs(format!("Invalid state: {}", state))),
        };
        
        let changed = match (current_state, target_state) {
            (PackageState::Present, PackageState::Present) => false,
            (PackageState::Absent, PackageState::Absent) => false,
            _ => true,
        };
        
        if context.check_mode {
            return Ok(ModuleResult {
                changed,
                failed: false,
                msg: Some(format!("Package {} would be {}", name, state)),
                stdout: None,
                stderr: None,
                rc: None,
                results: HashMap::new(),
                diff: None,
                warnings: Vec::new(),
                ansible_facts: HashMap::new(),
            });
        }
        
        if !changed {
            return Ok(ModuleResult {
                changed: false,
                failed: false,
                msg: Some(format!("Package {} is already {}", name, state)),
                stdout: None,
                stderr: None,
                rc: Some(0),
                results: HashMap::new(),
                diff: None,
                warnings: Vec::new(),
                ansible_facts: HashMap::new(),
            });
        }
        
        let result = match target_state {
            PackageState::Present => package_manager.install_package(name).await?,
            PackageState::Absent => package_manager.remove_package(name).await?,
        };
        
        Ok(ModuleResult {
            changed: true,
            failed: !result.success,
            msg: result.message,
            stdout: Some(result.stdout),
            stderr: Some(result.stderr),
            rc: Some(result.exit_code),
            results: HashMap::new(),
            diff: None,
            warnings: Vec::new(),
            ansible_facts: HashMap::new(),
        })
    }
    
    fn validate_args(&self, args: &ModuleArgs) -> Result<(), ValidationError> {
        if !args.args.contains_key("name") {
            return Err(ValidationError::MissingRequiredArg("name".to_string()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PackageState {
    Present,
    Absent,
}

#[async_trait]
pub trait PackageManager: Send + Sync {
    async fn query_package(&self, name: &str) -> Result<PackageState, PackageManagerError>;
    async fn install_package(&self, name: &str) -> Result<PackageResult, PackageManagerError>;
    async fn remove_package(&self, name: &str) -> Result<PackageResult, PackageManagerError>;
    async fn list_packages(&self) -> Result<Vec<Package>, PackageManagerError>;
}

#[derive(Debug, Clone)]
pub struct PackageResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
}

// Platform-specific package managers
pub struct AptPackageManager;
pub struct YumPackageManager;
pub struct BrewPackageManager;
pub struct ChocolateyPackageManager;
```

#### 4. Service Module
```rust
/// Service module - manages system services
pub struct ServiceModule {
    service_managers: HashMap<Platform, Box<dyn ServiceManager>>,
}

#[async_trait]
impl ExecutionModule for ServiceModule {
    fn name(&self) -> &'static str { "service" }
    fn version(&self) -> &'static str { "1.0.0" }
    fn supported_platforms(&self) -> &[Platform] { &[Platform::Linux, Platform::MacOS, Platform::Windows] }
    
    async fn execute(
        &self,
        args: &ModuleArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        let name = args.args.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ModuleExecutionError::InvalidArgs("name is required".to_string()))?;
        
        let state = args.args.get("state").and_then(|v| v.as_str());
        let enabled = args.args.get("enabled").and_then(|v| v.as_bool());
        
        let service_manager = self.service_managers.get(&context.host_info.platform)
            .ok_or_else(|| ModuleExecutionError::UnsupportedPlatform(context.host_info.platform.clone()))?;
        
        let current_status = service_manager.query_service(name).await?;
        let mut changed = false;
        let mut actions = Vec::new();
        
        // Handle state changes
        if let Some(target_state) = state {
            let target_running = match target_state {
                "started" | "running" => true,
                "stopped" => false,
                "restarted" | "reloaded" => {
                    // Always change for restart/reload
                    changed = true;
                    actions.push(target_state.to_string());
                    current_status.running  // Keep current state for now
                }
                _ => return Err(ModuleExecutionError::InvalidArgs(format!("Invalid state: {}", target_state))),
            };
            
            if target_state != "restarted" && target_state != "reloaded" && current_status.running != target_running {
                changed = true;
                actions.push(target_state.to_string());
            }
        }
        
        // Handle enabled changes
        if let Some(target_enabled) = enabled {
            if current_status.enabled != Some(target_enabled) {
                changed = true;
                actions.push(if target_enabled { "enable" } else { "disable" }.to_string());
            }
        }
        
        if context.check_mode {
            return Ok(ModuleResult {
                changed,
                failed: false,
                msg: Some(format!("Service {} would be modified: {:?}", name, actions)),
                stdout: None,
                stderr: None,
                rc: None,
                results: HashMap::new(),
                diff: None,
                warnings: Vec::new(),
                ansible_facts: HashMap::new(),
            });
        }
        
        if !changed {
            return Ok(ModuleResult {
                changed: false,
                failed: false,
                msg: Some(format!("Service {} is already in desired state", name)),
                stdout: None,
                stderr: None,
                rc: Some(0),
                results: HashMap::new(),
                diff: None,
                warnings: Vec::new(),
                ansible_facts: HashMap::new(),
            });
        }
        
        // Execute actions
        for action in &actions {
            let result = match action.as_str() {
                "started" | "running" => service_manager.start_service(name).await?,
                "stopped" => service_manager.stop_service(name).await?,
                "restarted" => service_manager.restart_service(name).await?,
                "reloaded" => service_manager.reload_service(name).await?,
                "enable" => service_manager.enable_service(name).await?,
                "disable" => service_manager.disable_service(name).await?,
                _ => continue,
            };
            
            if !result.success {
                return Ok(ModuleResult {
                    changed: true,
                    failed: true,
                    msg: Some(format!("Failed to {} service {}: {}", action, name, result.stderr)),
                    stdout: Some(result.stdout),
                    stderr: Some(result.stderr),
                    rc: Some(result.exit_code),
                    results: HashMap::new(),
                    diff: None,
                    warnings: Vec::new(),
                    ansible_facts: HashMap::new(),
                });
            }
        }
        
        Ok(ModuleResult {
            changed: true,
            failed: false,
            msg: Some(format!("Service {} successfully modified", name)),
            stdout: None,
            stderr: None,
            rc: Some(0),
            results: HashMap::new(),
            diff: None,
            warnings: Vec::new(),
            ansible_facts: HashMap::new(),
        })
    }
    
    fn validate_args(&self, args: &ModuleArgs) -> Result<(), ValidationError> {
        if !args.args.contains_key("name") {
            return Err(ValidationError::MissingRequiredArg("name".to_string()));
        }
        Ok(())
    }
}

#[async_trait]
pub trait ServiceManager: Send + Sync {
    async fn query_service(&self, name: &str) -> Result<ServiceStatus, ServiceManagerError>;
    async fn start_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError>;
    async fn stop_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError>;
    async fn restart_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError>;
    async fn reload_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError>;
    async fn enable_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError>;
    async fn disable_service(&self, name: &str) -> Result<ServiceResult, ServiceManagerError>;
}

#[derive(Debug, Clone)]
pub struct ServiceStatus {
    pub running: bool,
    pub enabled: Option<bool>,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct ServiceResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

// Platform-specific service managers
pub struct SystemdServiceManager;
pub struct InitServiceManager;
pub struct LaunchdServiceManager;
pub struct WindowsServiceManager;
```

### Module Registry and Factory

```rust
/// Central registry for all execution modules
pub struct ModuleRegistry {
    modules: HashMap<String, Box<dyn ExecutionModule>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            modules: HashMap::new(),
        };
        
        // Register core modules
        registry.register(Box::new(DebugModule));
        registry.register(Box::new(CommandModule));
        registry.register(Box::new(PackageModule::new()));
        registry.register(Box::new(ServiceModule::new()));
        
        registry
    }
    
    pub fn register(&mut self, module: Box<dyn ExecutionModule>) {
        self.modules.insert(module.name().to_string(), module);
    }
    
    pub fn get_module(&self, name: &str) -> Option<&dyn ExecutionModule> {
        self.modules.get(name).map(|m| m.as_ref())
    }
    
    pub fn list_modules(&self) -> Vec<&str> {
        self.modules.keys().map(|s| s.as_str()).collect()
    }
    
    pub async fn execute_module(
        &self,
        module_name: &str,
        args: &ModuleArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        let module = self.get_module(module_name)
            .ok_or_else(|| ModuleExecutionError::ModuleNotFound(module_name.to_string()))?;
        
        module.validate_args(args)?;
        module.execute(args, context).await
    }
}
```

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum ModuleExecutionError {
    #[error("Module not found: {0}")]
    ModuleNotFound(String),
    
    #[error("Invalid arguments: {0}")]
    InvalidArgs(String),
    
    #[error("Platform not supported: {0:?}")]
    UnsupportedPlatform(Platform),
    
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("Command parsing error: {0}")]
    CommandParsingError(#[from] shell_words::ParseError),
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Missing required argument: {0}")]
    MissingRequiredArg(String),
    
    #[error("Invalid argument type: {0}")]
    InvalidArgType(String),
    
    #[error("Invalid argument value: {0}")]
    InvalidArgValue(String),
}

#[derive(Debug, thiserror::Error)]
pub enum PackageManagerError {
    #[error("Package manager not available: {0}")]
    NotAvailable(String),
    
    #[error("Package not found: {0}")]
    PackageNotFound(String),
    
    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ServiceManagerError {
    #[error("Service manager not available: {0}")]
    NotAvailable(String),
    
    #[error("Service not found: {0}")]
    ServiceNotFound(String),
    
    #[error("Operation failed: {0}")]
    OperationFailed(String),
}
```

## File and Package Structure

```
src/modules/
├── mod.rs                     # Module exports and registry
├── core/
│   ├── mod.rs                 # Core module implementations
│   ├── debug.rs               # Debug module
│   ├── command.rs             # Command/shell execution
│   ├── package.rs             # Package management
│   ├── service.rs             # Service management
│   ├── copy.rs                # File copying
│   ├── template.rs            # Template rendering
│   ├── file.rs                # File operations
│   └── setup.rs               # Facts gathering
├── system/
│   ├── mod.rs                 # System integration modules
│   ├── package_managers/      # Platform-specific package managers
│   │   ├── apt.rs
│   │   ├── yum.rs
│   │   ├── brew.rs
│   │   └── chocolatey.rs
│   ├── service_managers/      # Platform-specific service managers
│   │   ├── systemd.rs
│   │   ├── init.rs
│   │   ├── launchd.rs
│   │   └── windows.rs
│   └── platform/              # Platform detection and utilities
│       ├── linux.rs
│       ├── macos.rs
│       └── windows.rs
├── registry.rs                # Module registry and factory
├── interface.rs               # Module interface traits
├── context.rs                 # Execution context
├── result.rs                  # Module result types
└── error.rs                   # Error types

tests/modules/
├── core/
│   ├── debug_tests.rs
│   ├── command_tests.rs
│   ├── package_tests.rs
│   └── service_tests.rs
├── integration/
│   ├── end_to_end_tests.rs
│   └── platform_tests.rs
└── fixtures/
    ├── test_playbooks/
    └── mock_systems/
```

## Implementation Details

### Phase 1: Core Module Framework
1. Implement ExecutionModule trait and base infrastructure
2. Create ModuleRegistry and factory pattern
3. Add DebugModule and CommandModule implementations
4. Create basic testing framework

### Phase 2: System Modules
1. Implement PackageModule with platform-specific managers
2. Add ServiceModule with service manager implementations
3. Create platform detection and abstraction
4. Add comprehensive error handling

### Phase 3: File and Template Modules
1. Implement CopyModule for file operations
2. Add TemplateModule with Jinja2 compatibility
3. Create FileModule for file management
4. Add diff generation and check mode support

### Phase 4: Advanced Features
1. Add privilege escalation (become) support
2. Implement facts collection and variable updates
3. Create module documentation system
4. Add performance optimization and caching

## Testing Strategy

### Unit Tests
- **Module Interface**: Test ExecutionModule trait implementations
- **Platform Abstraction**: Test package and service managers
- **Argument Validation**: Test module argument parsing and validation
- **Error Handling**: Test error conditions and recovery

### Integration Tests
- **Real System Tests**: Test modules against actual systems
- **Cross-Platform**: Test modules on Linux, macOS, and Windows
- **Ansible Compatibility**: Verify argument and behavior compatibility
- **Performance**: Benchmark module execution times

### Test Infrastructure
```
tests/fixtures/
├── systems/
│   ├── ubuntu_20_04/          # Test system configurations
│   ├── centos_8/
│   ├── macos_12/
│   └── windows_2019/
├── playbooks/
│   ├── core_modules.yml       # Test core module functionality
│   ├── package_tests.yml      # Package management tests
│   └── service_tests.yml      # Service management tests
└── expected_results/
    ├── debug_outputs.json
    ├── package_results.json
    └── service_results.json
```

## Dependencies

### External Crates
```toml
[dependencies]
async-trait = "0.1"
shell-words = "1.1"            # Command line parsing
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["process", "fs"] }
regex = "1.10"
uuid = { version = "1", features = ["v4"] }

# Platform-specific dependencies
[target.'cfg(unix)'.dependencies]
nix = "0.27"                   # Unix system calls

[target.'cfg(windows)'.dependencies]
winapi = { version = "0.3", features = ["winnt", "winsvc"] }
```

### Internal Dependencies
- `rustle_deploy::execution` - Execution context and planning
- `rustle_deploy::types` - Core type definitions
- `rustle_deploy::error` - Error handling patterns

## Configuration

### Module Configuration
```toml
[modules]
# Core modules always enabled
enable_core_modules = true

# Package manager preferences
[modules.package]
prefer_native = true           # Prefer native package managers
fallback_to_generic = false   # Don't fallback to generic managers

# Service manager preferences  
[modules.service]
prefer_systemd = true          # Prefer systemd on Linux
init_fallback = true           # Fallback to init scripts

# Command execution
[modules.command]
shell_timeout_secs = 300       # Default command timeout
max_output_size_mb = 10        # Limit command output size
```

### Environment Variables
- `RUSTLE_MODULE_TIMEOUT`: Default module execution timeout
- `RUSTLE_PACKAGE_MANAGER`: Force specific package manager
- `RUSTLE_SERVICE_MANAGER`: Force specific service manager
- `RUSTLE_MODULES_DEBUG`: Enable module debug logging

## Documentation

### Module Documentation
Each module provides comprehensive documentation including:
- Argument specifications
- Platform compatibility
- Usage examples
- Error conditions
- Performance characteristics

### Usage Examples
```rust
// Execute debug module
let args = ModuleArgs {
    args: hashmap! {
        "msg".to_string() => json!("Hello from Rustle!"),
    },
    special: SpecialParameters::default(),
};

let context = ExecutionContext {
    facts: HashMap::new(),
    variables: HashMap::new(),
    host_info: HostInfo::detect(),
    working_directory: PathBuf::from("/tmp"),
    environment: std::env::vars().collect(),
    check_mode: false,
    diff_mode: false,
    verbosity: 0,
};

let registry = ModuleRegistry::new();
let result = registry.execute_module("debug", &args, &context).await?;
```

## Integration Points

### Binary Deployment Integration
Modules are embedded into binary deployments through static compilation:

```rust
// Generate module registry for binary embedding
pub fn generate_embedded_module_registry() -> String {
    r#"
    use crate::modules::*;
    
    pub fn create_module_registry() -> ModuleRegistry {
        let mut registry = ModuleRegistry::new();
        
        // Core modules are always included
        registry.register(Box::new(DebugModule));
        registry.register(Box::new(CommandModule));
        registry.register(Box::new(PackageModule::new()));
        registry.register(Box::new(ServiceModule::new()));
        
        registry
    }
    "#
}
```

### Ansible Compatibility
Modules maintain 100% argument compatibility with Ansible:
- Same argument names and types
- Identical behavior and return values
- Compatible error messages and codes
- Matching idempotency semantics