# Spec 040: Runtime Execution Engine

## Feature Summary

Implement the runtime execution engine that gets embedded into compiled binaries to actually execute tasks from execution plans on target hosts. This is the core execution logic that runs locally on each target host after binary deployment, eliminating network overhead during execution.

**Problem it solves**: Generated binaries currently only print placeholder messages instead of executing actual tasks, making the entire deployment pipeline non-functional for real automation.

**High-level approach**: Create a lightweight, self-contained execution engine that can run tasks from various modules (debug, command, package, service, etc.), manage state, handle errors, and report progress back to the controller.

## Goals & Requirements

### Functional Requirements
- Execute tasks from embedded execution plans in proper order
- Support core Ansible-compatible modules (debug, command, shell, copy, template, package, service)
- Handle task dependencies and conditional execution
- Manage task state and collect results
- Report execution progress and results
- Handle task failures with configurable policies
- Support parallel task execution where possible
- Collect and cache host facts
- Handle task retries and timeouts
- Support custom module execution

### Non-functional Requirements
- **Performance**: Execute 1000+ tasks in <5 minutes on typical hardware
- **Memory**: Use <100MB memory for typical workloads
- **Reliability**: 99.9%+ task execution success rate for valid tasks
- **Portability**: Run on Linux, macOS, Windows without external dependencies
- **Size**: Keep execution engine <5MB when embedded

### Success Criteria
- Successfully execute all supported task types
- Maintain task execution order and dependencies
- Provide real-time execution progress
- Handle common failure scenarios gracefully
- Support both sequential and parallel execution modes

## API/Interface Design

### Core Execution Engine

```rust
/// Main execution engine for embedded execution plans
pub struct LocalExecutor {
    config: RuntimeConfig,
    module_registry: ModuleRegistry,
    facts_cache: FactsCache,
    state_manager: StateManager,
    progress_reporter: ProgressReporter,
}

impl LocalExecutor {
    pub fn new(config: RuntimeConfig) -> Self;
    
    pub async fn execute_plan(&self, plan: ExecutionPlan) -> Result<ExecutionResult, ExecutionError>;
    
    pub async fn execute_play(&self, play: &Play) -> Result<PlayResult, ExecutionError>;
    
    pub async fn execute_batch(&self, batch: &TaskBatch) -> Result<BatchResult, ExecutionError>;
    
    pub async fn execute_task(&self, task: &Task) -> Result<TaskResult, ExecutionError>;
    
    pub fn collect_facts(&self) -> Result<HashMap<String, serde_json::Value>, FactsError>;
    
    pub async fn report_progress(&self, progress: ExecutionProgress) -> Result<(), ReportError>;
    
    pub fn cleanup(&self) -> Result<(), CleanupError>;
}

/// Task execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub name: String,
    pub status: TaskStatus,
    pub changed: bool,
    pub failed: bool,
    pub skipped: bool,
    pub output: serde_json::Value,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration: Duration,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Success,
    Failed,
    Skipped,
    Timeout,
    Cancelled,
}

/// Execution state management
pub struct StateManager {
    task_results: HashMap<String, TaskResult>,
    execution_state: ExecutionState,
    facts: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionState {
    pub current_play: Option<String>,
    pub current_task: Option<String>,
    pub failed_tasks: Vec<String>,
    pub changed_tasks: Vec<String>,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub start_time: DateTime<Utc>,
}

/// Progress reporting for controller communication
pub struct ProgressReporter {
    controller_endpoint: Option<String>,
    client: Option<reqwest::Client>,
}

impl ProgressReporter {
    pub async fn report_task_start(&self, task: &Task) -> Result<(), ReportError>;
    pub async fn report_task_complete(&self, result: &TaskResult) -> Result<(), ReportError>;
    pub async fn report_execution_complete(&self, result: &ExecutionResult) -> Result<(), ReportError>;
    pub async fn report_error(&self, error: &ExecutionError) -> Result<(), ReportError>;
}
```

### Module System

```rust
/// Registry for task execution modules
pub struct ModuleRegistry {
    modules: HashMap<String, Box<dyn Module>>,
}

impl ModuleRegistry {
    pub fn new() -> Self;
    pub fn register_builtin_modules(&mut self);
    pub fn register_module(&mut self, name: String, module: Box<dyn Module>);
    pub fn get_module(&self, name: &str) -> Option<&dyn Module>;
    pub fn execute_task(&self, task: &Task, context: &ExecutionContext) -> Result<TaskResult, ModuleError>;
}

/// Core module trait for task execution
pub trait Module: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, args: &HashMap<String, serde_json::Value>, context: &ExecutionContext) -> Result<ModuleResult, ModuleError>;
    fn validate_args(&self, args: &HashMap<String, serde_json::Value>) -> Result<(), ValidationError>;
    fn required_args(&self) -> Vec<&str>;
    fn optional_args(&self) -> Vec<&str>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleResult {
    pub changed: bool,
    pub failed: bool,
    pub output: serde_json::Value,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub message: Option<String>,
}

pub struct ExecutionContext {
    pub facts: HashMap<String, serde_json::Value>,
    pub variables: HashMap<String, serde_json::Value>,
    pub previous_results: HashMap<String, TaskResult>,
    pub working_directory: PathBuf,
    pub check_mode: bool,
}
```

### Built-in Modules

```rust
/// Debug module for testing and information display
pub struct DebugModule;

impl Module for DebugModule {
    fn name(&self) -> &str { "debug" }
    
    fn execute(&self, args: &HashMap<String, serde_json::Value>, _context: &ExecutionContext) -> Result<ModuleResult, ModuleError> {
        let msg = args.get("msg")
            .and_then(|v| v.as_str())
            .unwrap_or("Debug message");
        
        println!("{}", msg);
        
        Ok(ModuleResult {
            changed: false,
            failed: false,
            output: json!({ "msg": msg }),
            stdout: Some(msg.to_string()),
            stderr: None,
            message: Some(msg.to_string()),
        })
    }
}

/// Command execution module
pub struct CommandModule;

impl Module for CommandModule {
    fn name(&self) -> &str { "command" }
    
    fn execute(&self, args: &HashMap<String, serde_json::Value>, context: &ExecutionContext) -> Result<ModuleResult, ModuleError> {
        let cmd = args.get("_raw_params")
            .or_else(|| args.get("cmd"))
            .and_then(|v| v.as_str())
            .ok_or(ModuleError::MissingRequiredArg { arg: "cmd".to_string() })?;
        
        let working_dir = args.get("chdir")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| context.working_directory.clone());
        
        let timeout = args.get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(300); // Default 5 minute timeout
        
        if context.check_mode {
            return Ok(ModuleResult {
                changed: true,
                failed: false,
                output: json!({ "cmd": cmd, "check_mode": true }),
                stdout: None,
                stderr: None,
                message: Some(format!("Would run: {}", cmd)),
            });
        }
        
        let output = tokio::time::timeout(
            Duration::from_secs(timeout),
            tokio::process::Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .current_dir(working_dir)
                .output()
        ).await??;
        
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let success = output.status.success();
        
        Ok(ModuleResult {
            changed: success,
            failed: !success,
            output: json!({
                "cmd": cmd,
                "rc": output.status.code().unwrap_or(-1),
                "stdout": stdout,
                "stderr": stderr,
            }),
            stdout: Some(stdout),
            stderr: Some(stderr),
            message: None,
        })
    }
}

/// Package management module
pub struct PackageModule;

impl Module for PackageModule {
    fn name(&self) -> &str { "package" }
    
    fn execute(&self, args: &HashMap<String, serde_json::Value>, context: &ExecutionContext) -> Result<ModuleResult, ModuleError> {
        let name = args.get("name")
            .and_then(|v| v.as_str())
            .ok_or(ModuleError::MissingRequiredArg { arg: "name".to_string() })?;
        
        let state = args.get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("present");
        
        let package_manager = self.detect_package_manager()?;
        
        match state {
            "present" | "installed" => self.install_package(name, &package_manager, context),
            "absent" | "removed" => self.remove_package(name, &package_manager, context),
            "latest" => self.update_package(name, &package_manager, context),
            _ => Err(ModuleError::InvalidArg { 
                arg: "state".to_string(), 
                value: state.to_string() 
            }),
        }
    }
    
    fn detect_package_manager(&self) -> Result<PackageManager, ModuleError> {
        // Detect available package manager
        if std::process::Command::new("apt").arg("--version").output().is_ok() {
            Ok(PackageManager::Apt)
        } else if std::process::Command::new("yum").arg("--version").output().is_ok() {
            Ok(PackageManager::Yum)
        } else if std::process::Command::new("dnf").arg("--version").output().is_ok() {
            Ok(PackageManager::Dnf)
        } else if std::process::Command::new("brew").arg("--version").output().is_ok() {
            Ok(PackageManager::Brew)
        } else {
            Err(ModuleError::UnsupportedPlatform { 
                reason: "No supported package manager found".to_string() 
            })
        }
    }
}

#[derive(Debug)]
enum PackageManager {
    Apt,
    Yum,
    Dnf,
    Brew,
    Pacman,
    Zypper,
}

/// Service management module
pub struct ServiceModule;

/// Copy files module
pub struct CopyModule;

/// Template processing module  
pub struct TemplateModule;
```

### Facts Collection

```rust
/// System facts collector
pub struct FactsCollector;

impl FactsCollector {
    pub fn collect_all_facts() -> Result<HashMap<String, serde_json::Value>, FactsError> {
        let mut facts = HashMap::new();
        
        facts.extend(Self::collect_system_facts()?);
        facts.extend(Self::collect_network_facts()?);
        facts.extend(Self::collect_platform_facts()?);
        facts.extend(Self::collect_hardware_facts()?);
        
        Ok(facts)
    }
    
    fn collect_system_facts() -> Result<HashMap<String, serde_json::Value>, FactsError> {
        let mut facts = HashMap::new();
        
        // Basic system information
        facts.insert("ansible_hostname".to_string(), json!(hostname::get()?.to_string_lossy()));
        facts.insert("ansible_fqdn".to_string(), json!(Self::get_fqdn()?));
        facts.insert("ansible_os_family".to_string(), json!(Self::get_os_family()));
        facts.insert("ansible_system".to_string(), json!(std::env::consts::OS));
        facts.insert("ansible_architecture".to_string(), json!(std::env::consts::ARCH));
        
        // Kernel and version information
        if let Ok(uname) = Self::run_command("uname -r") {
            facts.insert("ansible_kernel".to_string(), json!(uname.trim()));
        }
        
        // Distribution information (Linux)
        if cfg!(target_os = "linux") {
            facts.extend(Self::collect_linux_distribution_facts()?);
        }
        
        Ok(facts)
    }
    
    fn collect_network_facts() -> Result<HashMap<String, serde_json::Value>, FactsError> {
        let mut facts = HashMap::new();
        
        // Get network interfaces
        let interfaces = Self::get_network_interfaces()?;
        facts.insert("ansible_interfaces".to_string(), json!(interfaces));
        
        // Get default gateway and routes
        if let Ok(default_ipv4) = Self::get_default_route() {
            facts.insert("ansible_default_ipv4".to_string(), json!(default_ipv4));
        }
        
        Ok(facts)
    }
    
    fn get_os_family() -> &'static str {
        match std::env::consts::OS {
            "linux" => "RedHat", // Could be more specific
            "macos" => "Darwin",
            "windows" => "Windows",
            _ => "Unknown",
        }
    }
    
    fn run_command(cmd: &str) -> Result<String, FactsError> {
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()?;
        
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(FactsError::CommandFailed {
                command: cmd.to_string(),
                error: String::from_utf8_lossy(&output.stderr).to_string(),
            })
        }
    }
}

/// Cache for facts to avoid repeated collection
pub struct FactsCache {
    cache: HashMap<String, CachedFact>,
    ttl: Duration,
}

#[derive(Debug, Clone)]
struct CachedFact {
    value: serde_json::Value,
    collected_at: Instant,
}

impl FactsCache {
    pub fn new(ttl: Duration) -> Self;
    pub fn get(&self, key: &str) -> Option<&serde_json::Value>;
    pub fn set(&mut self, key: String, value: serde_json::Value);
    pub fn invalidate(&mut self, key: &str);
    pub fn clear_expired(&mut self);
}
```

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Task execution failed: {task_id}")]
    TaskFailed { task_id: String, reason: String },
    
    #[error("Module not found: {module}")]
    ModuleNotFound { module: String },
    
    #[error("Dependency cycle detected: {cycle:?}")]
    DependencyCycle { cycle: Vec<String> },
    
    #[error("Task timeout: {task_id} ({timeout}s)")]
    TaskTimeout { task_id: String, timeout: u64 },
    
    #[error("Condition evaluation failed: {condition}")]
    ConditionFailed { condition: String },
    
    #[error("Facts collection failed: {reason}")]
    FactsCollectionFailed { reason: String },
    
    #[error("Controller communication failed: {reason}")]
    ControllerCommunicationFailed { reason: String },
    
    #[error("Invalid execution plan: {reason}")]
    InvalidExecutionPlan { reason: String },
}

#[derive(Debug, thiserror::Error)]
pub enum ModuleError {
    #[error("Missing required argument: {arg}")]
    MissingRequiredArg { arg: String },
    
    #[error("Invalid argument: {arg} = {value}")]
    InvalidArg { arg: String, value: String },
    
    #[error("Command execution failed: {command}")]
    CommandFailed { command: String, exit_code: i32 },
    
    #[error("Permission denied: {operation}")]
    PermissionDenied { operation: String },
    
    #[error("File not found: {path}")]
    FileNotFound { path: String },
    
    #[error("Unsupported platform: {reason}")]
    UnsupportedPlatform { reason: String },
    
    #[error("Network error: {reason}")]
    NetworkError { reason: String },
    
    #[error("Timeout: {operation} ({timeout}s)")]
    Timeout { operation: String, timeout: u64 },
}

#[derive(Debug, thiserror::Error)]
pub enum FactsError {
    #[error("Command failed: {command} - {error}")]
    CommandFailed { command: String, error: String },
    
    #[error("Parse error: {reason}")]
    ParseError { reason: String },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("System error: {reason}")]
    SystemError { reason: String },
}
```

## File and Package Structure

```
src/runtime/
├── mod.rs                     # Module exports
├── executor.rs                # LocalExecutor implementation
├── state.rs                   # State management
├── progress.rs                # Progress reporting
├── facts.rs                   # Facts collection
├── conditions.rs              # Condition evaluation
└── error.rs                   # Error types

src/modules/
├── mod.rs                     # Module registry
├── registry.rs                # ModuleRegistry implementation
├── debug.rs                   # Debug module
├── command.rs                 # Command execution
├── shell.rs                   # Shell command execution
├── copy.rs                    # File copying
├── template.rs                # Template processing
├── package.rs                 # Package management
├── service.rs                 # Service management
├── file.rs                    # File operations
└── setup.rs                   # System setup facts

templates/
├── runner_main.rs.template    # Template for generated main.rs
└── runner_cargo.toml.template # Template for generated Cargo.toml

tests/runtime/
├── executor_tests.rs
├── module_tests.rs
├── facts_tests.rs
├── integration_tests.rs
└── fixtures/
    ├── test_plans/
    ├── test_modules/
    └── test_facts/
```

## Implementation Details

### Phase 1: Core Execution Engine
1. Implement LocalExecutor and basic task execution loop
2. Create module registry and trait system
3. Add basic built-in modules (debug, command)
4. Implement state management and progress tracking

### Phase 2: Module Implementation
1. Add remaining core modules (package, service, copy, template)
2. Implement facts collection system
3. Add condition evaluation logic
4. Create progress reporting to controller

### Phase 3: Advanced Features
1. Add parallel task execution support
2. Implement retry mechanisms and timeout handling
3. Add custom module loading capabilities
4. Create comprehensive error recovery

### Key Algorithms

**Task Execution with Dependencies**:
```rust
impl LocalExecutor {
    pub async fn execute_batch(&self, batch: &TaskBatch) -> Result<BatchResult, ExecutionError> {
        let mut results = HashMap::new();
        let mut completed = HashSet::new();
        let mut failed = HashSet::new();
        
        // Build dependency graph
        let dependency_graph = self.build_dependency_graph(&batch.tasks)?;
        
        // Execute tasks in topological order
        while completed.len() + failed.len() < batch.tasks.len() {
            let ready_tasks = self.find_ready_tasks(&batch.tasks, &dependency_graph, &completed, &failed);
            
            if ready_tasks.is_empty() {
                return Err(ExecutionError::DependencyCycle { 
                    cycle: self.find_remaining_tasks(&batch.tasks, &completed, &failed)
                });
            }
            
            // Execute ready tasks (potentially in parallel)
            let task_futures: Vec<_> = ready_tasks.into_iter()
                .map(|task| self.execute_single_task(task))
                .collect();
            
            let task_results = futures::future::join_all(task_futures).await;
            
            for result in task_results {
                match result {
                    Ok(task_result) => {
                        let task_id = task_result.task_id.clone();
                        if task_result.failed {
                            failed.insert(task_id.clone());
                        } else {
                            completed.insert(task_id.clone());
                        }
                        results.insert(task_id, task_result);
                    }
                    Err(e) => {
                        // Handle task execution error
                        return Err(e);
                    }
                }
            }
        }
        
        Ok(BatchResult {
            task_results: results,
            overall_success: failed.is_empty(),
            duration: start_time.elapsed(),
        })
    }
    
    async fn execute_single_task(&self, task: &Task) -> Result<TaskResult, ExecutionError> {
        let start_time = Instant::now();
        
        // Report task start
        self.progress_reporter.report_task_start(task).await?;
        
        // Evaluate conditions
        if !self.evaluate_conditions(&task.conditions)? {
            let result = TaskResult {
                task_id: task.id.clone(),
                name: task.name.clone(),
                status: TaskStatus::Skipped,
                changed: false,
                failed: false,
                skipped: true,
                output: json!({"skipped": true, "reason": "Condition not met"}),
                stdout: None,
                stderr: None,
                start_time: Utc::now(),
                end_time: Utc::now(),
                duration: Duration::from_millis(0),
                error: None,
            };
            
            self.progress_reporter.report_task_complete(&result).await?;
            return Ok(result);
        }
        
        // Execute the task
        let execution_context = ExecutionContext {
            facts: self.facts_cache.get_all_facts(),
            variables: HashMap::new(), // TODO: Populate from execution plan
            previous_results: self.state_manager.get_task_results(),
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            check_mode: self.config.check_mode.unwrap_or(false),
        };
        
        let module_result = match self.config.task_timeout {
            Some(timeout) => {
                tokio::time::timeout(
                    timeout,
                    self.module_registry.execute_task(task, &execution_context)
                ).await??
            }
            None => {
                self.module_registry.execute_task(task, &execution_context)?
            }
        };
        
        let task_result = TaskResult {
            task_id: task.id.clone(),
            name: task.name.clone(),
            status: if module_result.failed { TaskStatus::Failed } else { TaskStatus::Success },
            changed: module_result.changed,
            failed: module_result.failed,
            skipped: false,
            output: module_result.output,
            stdout: module_result.stdout,
            stderr: module_result.stderr,
            start_time: Utc::now() - chrono::Duration::from_std(start_time.elapsed()).unwrap(),
            end_time: Utc::now(),
            duration: start_time.elapsed(),
            error: None,
        };
        
        // Report task completion
        self.progress_reporter.report_task_complete(&task_result).await?;
        
        Ok(task_result)
    }
}
```

**Template Generation for Runtime**:
```rust
impl BinaryCompiler {
    fn generate_runtime_main(&self, compilation: &BinaryCompilation) -> Result<String, CompileError> {
        let template = r#"
use std::collections::HashMap;
use serde_json::Value;
use anyhow::Result;

mod embedded_data {
    pub const EXECUTION_PLAN: &str = {{execution_plan}};
    pub const RUNTIME_CONFIG: &str = {{runtime_config}};
}

mod runtime {
    use super::*;
    
    // Include the embedded runtime execution engine
    include!("runtime/executor.rs");
    include!("runtime/state.rs");
    include!("runtime/progress.rs");
    include!("runtime/facts.rs");
    include!("modules/registry.rs");
    include!("modules/debug.rs");
    include!("modules/command.rs");
    include!("modules/package.rs");
    include!("modules/service.rs");
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    // Parse embedded execution plan
    let execution_plan: ExecutionPlan = serde_json::from_str(embedded_data::EXECUTION_PLAN)?;
    let runtime_config: RuntimeConfig = serde_json::from_str(embedded_data::RUNTIME_CONFIG)?;
    
    // Create and run executor
    let executor = runtime::LocalExecutor::new(runtime_config);
    let result = executor.execute_plan(execution_plan).await?;
    
    // Report final results
    if let Some(controller) = executor.config.controller_endpoint {
        executor.report_execution_complete(&result).await?;
    }
    
    // Exit with appropriate code
    if result.failed {
        std::process::exit(1);
    } else {
        std::process::exit(0);
    }
}
"#;
        
        let rendered = template
            .replace("{{execution_plan}}", &serde_json::to_string(&compilation.embedded_data.execution_plan)?)
            .replace("{{runtime_config}}", &serde_json::to_string(&compilation.embedded_data.runtime_config)?);
        
        Ok(rendered)
    }
}
```

## Testing Strategy

### Unit Tests
- **Module Tests**: Each module with various argument combinations
- **Executor Tests**: Task execution, dependency handling, error scenarios
- **Facts Tests**: Facts collection on different platforms
- **State Tests**: State management and persistence

### Integration Tests
- **End-to-end**: Complete execution plan processing
- **Multi-platform**: Runtime execution on Linux, macOS, Windows
- **Performance**: Large task execution benchmarks
- **Error Handling**: Various failure scenarios and recovery

### Test Data
```
tests/fixtures/runtime/
├── execution_plans/
│   ├── simple_debug.json      # Basic debug tasks
│   ├── command_execution.json # Command module testing
│   ├── package_install.json   # Package management
│   ├── service_control.json   # Service management
│   ├── dependencies.json      # Task dependency chains
│   └── parallel_tasks.json    # Parallel execution
├── modules/
│   ├── custom_module.rs       # Custom module for testing
│   └── failing_module.rs      # Module that always fails
└── expected_results/
    ├── debug_output.json
    ├── command_output.json
    └── package_output.json
```

## Edge Cases & Error Handling

### Execution Edge Cases
- Task dependency cycles
- Long-running tasks and timeouts
- Resource exhaustion (memory, disk, network)
- Platform-specific failures
- Permission denied scenarios

### Module Edge Cases
- Invalid command syntax
- Non-existent packages or services
- Network connectivity issues
- File system permission problems
- Concurrent access conflicts

### Recovery Strategies
- Graceful task failure handling
- Retry mechanisms with exponential backoff
- Partial execution state preservation
- Rollback capabilities for critical failures
- Resource cleanup on abnormal termination

## Dependencies

### External Crates
```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
serde_json = "1"
reqwest = { version = "0.11", features = ["json"] }
futures = "0.3"
hostname = "0.3"
which = "4.4"        # Finding executables
nix = "0.27"         # Unix system calls (Linux/macOS)
winapi = "0.3"       # Windows API (Windows)
sysinfo = "0.29"     # System information
```

### Conditional Platform Dependencies
```toml
[target.'cfg(unix)'.dependencies]
nix = "0.27"

[target.'cfg(windows)'.dependencies]
winapi = { version = "0.3", features = ["processthreadsapi", "winuser"] }
windows = "0.48"
```

## Configuration

### Runtime Configuration
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub controller_endpoint: Option<String>,
    pub execution_timeout: Duration,
    pub task_timeout: Option<Duration>,
    pub report_interval: Duration,
    pub cleanup_on_completion: bool,
    pub log_level: String,
    pub check_mode: Option<bool>,
    pub parallel_tasks: Option<usize>,
    pub facts_cache_ttl: Duration,
    pub retry_policy: Option<RetryPolicy>,
}
```

## Documentation

### Module Development Guide
```rust
/// Custom module implementation example
pub struct CustomModule;

impl Module for CustomModule {
    fn name(&self) -> &str { "custom" }
    
    fn description(&self) -> &str { "Custom task execution module" }
    
    fn required_args(&self) -> Vec<&str> { vec!["action"] }
    
    fn optional_args(&self) -> Vec<&str> { vec!["timeout", "retry"] }
    
    fn execute(&self, args: &HashMap<String, Value>, context: &ExecutionContext) -> Result<ModuleResult, ModuleError> {
        // Implementation
    }
}
```

### Integration Examples
```rust
// Execution in generated binary
let executor = LocalExecutor::new(runtime_config);
let result = executor.execute_plan(execution_plan).await?;

// Custom module registration
let mut registry = ModuleRegistry::new();
registry.register_builtin_modules();
registry.register_module("custom".to_string(), Box::new(CustomModule));
```