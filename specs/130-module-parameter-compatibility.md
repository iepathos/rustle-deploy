# 130-module-parameter-compatibility

## Feature Summary

Fix module parameter compatibility issues where execution plans use different parameter names than what the module implementations expect. Currently, the command module expects `cmd` or `command` parameters but Ansible/rustle-plan uses `_raw_params`, causing tasks to fail with "Missing 'cmd' or 'command' parameter" errors.

This spec addresses the broader issue of parameter name mismatches between Ansible conventions and the current module implementations, ensuring seamless execution of generated plans.

## Goals & Requirements

### Functional Requirements
- **FR1**: Support Ansible-standard parameter names in all core modules (command, shell, package, service, debug)
- **FR2**: Maintain backward compatibility with existing parameter names  
- **FR3**: Provide clear parameter mapping documentation for module developers
- **FR4**: Handle both positional (`_raw_params`) and named parameters appropriately
- **FR5**: Support module-specific parameter aliases (e.g., `name` vs `pkg` for package modules)

### Non-functional Requirements
- **NFR1**: Zero performance impact from parameter mapping
- **NFR2**: Clear error messages when required parameters are missing
- **NFR3**: Consistent parameter handling across all modules
- **NFR4**: Easy extensibility for future modules

### Success Criteria
- All tasks in `example_rustle_plan_output.json` execute successfully
- Command module handles `_raw_params` parameter correctly
- Package module supports both `name` and `pkg` parameters
- Service module supports standard Ansible parameter names
- Debug module handles `msg` and `var` parameters appropriately

## API/Interface Design

### Enhanced Module Parameter Handler
```rust
pub trait ModuleParameterHandler {
    /// Map Ansible-style parameters to module-expected parameters
    fn map_parameters(&self, ansible_params: HashMap<String, Value>) -> Result<HashMap<String, Value>, ParameterError>;
    
    /// Get required parameters for this module
    fn required_parameters(&self) -> Vec<&'static str>;
    
    /// Get parameter aliases for this module
    fn parameter_aliases(&self) -> HashMap<&'static str, Vec<&'static str>>;
    
    /// Validate that all required parameters are present
    fn validate_parameters(&self, params: &HashMap<String, Value>) -> Result<(), ParameterError>;
}

#[derive(Error, Debug)]
pub enum ParameterError {
    #[error("Missing required parameter: {param}")]
    MissingRequired { param: String },
    
    #[error("Invalid parameter value for {param}: {reason}")]
    InvalidValue { param: String, reason: String },
    
    #[error("Conflicting parameters: {params:?}")]
    ConflictingParameters { params: Vec<String> },
    
    #[error("Unknown parameter: {param}")]
    UnknownParameter { param: String },
}
```

### Parameter Mapping Utilities
```rust
pub struct ParameterMapper {
    module_handlers: HashMap<String, Box<dyn ModuleParameterHandler>>,
}

impl ParameterMapper {
    pub fn new() -> Self {
        let mut handlers: HashMap<String, Box<dyn ModuleParameterHandler>> = HashMap::new();
        
        handlers.insert("command".to_string(), Box::new(CommandParameterHandler));
        handlers.insert("shell".to_string(), Box::new(CommandParameterHandler));
        handlers.insert("package".to_string(), Box::new(PackageParameterHandler));
        handlers.insert("service".to_string(), Box::new(ServiceParameterHandler));
        handlers.insert("debug".to_string(), Box::new(DebugParameterHandler));
        
        Self { module_handlers: handlers }
    }
    
    pub fn map_for_module(&self, module_name: &str, params: HashMap<String, Value>) -> Result<HashMap<String, Value>, ParameterError> {
        let handler = self.module_handlers.get(module_name)
            .ok_or_else(|| ParameterError::UnknownParameter { 
                param: format!("module: {}", module_name) 
            })?;
            
        let mapped = handler.map_parameters(params)?;
        handler.validate_parameters(&mapped)?;
        
        Ok(mapped)
    }
}
```

### Module-Specific Parameter Handlers

#### Command Module Handler
```rust
pub struct CommandParameterHandler;

impl ModuleParameterHandler for CommandParameterHandler {
    fn map_parameters(&self, mut ansible_params: HashMap<String, Value>) -> Result<HashMap<String, Value>, ParameterError> {
        let mut mapped = HashMap::new();
        
        // Handle _raw_params -> cmd mapping
        if let Some(raw_params) = ansible_params.remove("_raw_params") {
            mapped.insert("cmd".to_string(), raw_params);
        }
        
        // Handle existing cmd/command parameters
        if let Some(cmd) = ansible_params.remove("cmd") {
            mapped.insert("cmd".to_string(), cmd);
        } else if let Some(command) = ansible_params.remove("command") {
            mapped.insert("cmd".to_string(), command);
        }
        
        // Handle chdir parameter
        if let Some(chdir) = ansible_params.remove("chdir") {
            mapped.insert("chdir".to_string(), chdir);
        }
        
        // Handle creates/removes parameters
        if let Some(creates) = ansible_params.remove("creates") {
            mapped.insert("creates".to_string(), creates);
        }
        if let Some(removes) = ansible_params.remove("removes") {
            mapped.insert("removes".to_string(), removes);
        }
        
        // Pass through any other parameters
        for (key, value) in ansible_params {
            mapped.insert(key, value);
        }
        
        Ok(mapped)
    }
    
    fn required_parameters(&self) -> Vec<&'static str> {
        vec!["cmd"]
    }
    
    fn parameter_aliases(&self) -> HashMap<&'static str, Vec<&'static str>> {
        let mut aliases = HashMap::new();
        aliases.insert("cmd", vec!["command", "_raw_params"]);
        aliases
    }
    
    fn validate_parameters(&self, params: &HashMap<String, Value>) -> Result<(), ParameterError> {
        if !params.contains_key("cmd") {
            return Err(ParameterError::MissingRequired { 
                param: "cmd (or _raw_params, command)".to_string() 
            });
        }
        Ok(())
    }
}
```

#### Package Module Handler  
```rust
pub struct PackageParameterHandler;

impl ModuleParameterHandler for PackageParameterHandler {
    fn map_parameters(&self, mut ansible_params: HashMap<String, Value>) -> Result<HashMap<String, Value>, ParameterError> {
        let mut mapped = HashMap::new();
        
        // Handle name/pkg parameter aliases
        if let Some(name) = ansible_params.remove("name") {
            mapped.insert("name".to_string(), name);
        } else if let Some(pkg) = ansible_params.remove("pkg") {
            mapped.insert("name".to_string(), pkg);
        }
        
        // Handle state parameter (default to present)
        let state = ansible_params.remove("state")
            .unwrap_or_else(|| Value::String("present".to_string()));
        mapped.insert("state".to_string(), state);
        
        // Handle version parameter
        if let Some(version) = ansible_params.remove("version") {
            mapped.insert("version".to_string(), version);
        }
        
        // Pass through other parameters
        for (key, value) in ansible_params {
            mapped.insert(key, value);
        }
        
        Ok(mapped)
    }
    
    fn required_parameters(&self) -> Vec<&'static str> {
        vec!["name"]
    }
    
    fn parameter_aliases(&self) -> HashMap<&'static str, Vec<&'static str>> {
        let mut aliases = HashMap::new();
        aliases.insert("name", vec!["pkg"]);
        aliases
    }
    
    fn validate_parameters(&self, params: &HashMap<String, Value>) -> Result<(), ParameterError> {
        if !params.contains_key("name") {
            return Err(ParameterError::MissingRequired { 
                param: "name (or pkg)".to_string() 
            });
        }
        Ok(())
    }
}
```

### Enhanced Module Execution
```rust
// Update main.rs template module execution
async fn execute_task(&mut self, task: &TaskPlan) -> Result<TaskResult> {
    let start_time = std::time::SystemTime::now();
    let execution_start = std::time::Instant::now();
    
    // Map parameters using ParameterMapper
    let parameter_mapper = ParameterMapper::new();
    let mapped_args = parameter_mapper.map_for_module(&task.module, task.args.clone())
        .map_err(|e| anyhow::anyhow!("Parameter mapping failed: {}", e))?;
    
    // Execute module with mapped parameters
    let module_result_value = match task.module.as_str() {
        "command" | "shell" => {
            modules::command::execute(mapped_args).await?
        }
        "package" => {
            modules::package::execute(mapped_args).await?
        }
        "service" => {
            modules::service::execute(mapped_args).await?
        }
        "debug" => {
            modules::debug::execute(mapped_args).await?
        }
        _ => {
            return Err(anyhow::anyhow!("Unsupported module: {}", task.module));
        }
    };
    
    // ... rest of execution logic unchanged ...
}
```

## File and Package Structure

```
src/templates/modules/
├── mod.rs                        # Module exports and ParameterMapper
├── parameter_mapping/
│   ├── mod.rs                    # Parameter mapping exports
│   ├── mapper.rs                 # ParameterMapper implementation
│   ├── handlers/
│   │   ├── mod.rs                # Handler exports
│   │   ├── command.rs            # CommandParameterHandler
│   │   ├── package.rs            # PackageParameterHandler
│   │   ├── service.rs            # ServiceParameterHandler
│   │   └── debug.rs              # DebugParameterHandler
│   └── error.rs                  # ParameterError types
├── command.rs                    # Enhanced command module
├── package.rs                    # Enhanced package module  
├── service.rs                    # Enhanced service module
└── debug.rs                      # Enhanced debug module
```

### Updated Main Template
The `main_rs.template` will be updated to include the parameter mapping logic in the module execution section.

## Implementation Details

### Step 1: Create Parameter Mapping Infrastructure
```rust
// src/templates/modules/parameter_mapping/mapper.rs
impl ParameterMapper {
    pub fn map_for_module(&self, module_name: &str, params: HashMap<String, Value>) -> Result<HashMap<String, Value>, ParameterError> {
        tracing::debug!("Mapping parameters for module '{}': {:?}", module_name, params);
        
        let handler = self.module_handlers.get(module_name)
            .ok_or_else(|| ParameterError::UnknownParameter { 
                param: format!("module: {}", module_name) 
            })?;
            
        let mapped = handler.map_parameters(params)?;
        handler.validate_parameters(&mapped)?;
        
        tracing::debug!("Mapped parameters: {:?}", mapped);
        Ok(mapped)
    }
}
```

### Step 2: Enhance Command Module
```rust
// Update src/templates/modules/command.rs
pub async fn execute(args: HashMap<String, Value>) -> Result<Value> {
    // Parameters are now guaranteed to be mapped correctly
    let cmd = args.get("cmd")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'cmd' parameter"))?;
    
    let chdir = args.get("chdir").and_then(|v| v.as_str());
    let creates = args.get("creates").and_then(|v| v.as_str());
    let removes = args.get("removes").and_then(|v| v.as_str());
    
    // Check creates/removes conditions
    if let Some(creates_path) = creates {
        if std::path::Path::new(creates_path).exists() {
            return Ok(serde_json::json!({
                "changed": false,
                "failed": false,
                "skipped": true,
                "msg": format!("skipped, since {} exists", creates_path),
            }));
        }
    }
    
    if let Some(removes_path) = removes {
        if !std::path::Path::new(removes_path).exists() {
            return Ok(serde_json::json!({
                "changed": false,
                "failed": false,
                "skipped": true,
                "msg": format!("skipped, since {} does not exist", removes_path),
            }));
        }
    }
    
    let mut command = if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
    } else {
        std::process::Command::new("sh")
    };
    
    if cfg!(target_os = "windows") {
        command.args(&["/C", cmd]);
    } else {
        command.args(&["-c", cmd]);
    }
    
    // Handle chdir
    if let Some(dir) = chdir {
        command.current_dir(dir);
    }
    
    let output = command.output()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let rc = output.status.code().unwrap_or(-1);
    
    Ok(serde_json::json!({
        "changed": rc == 0,
        "failed": rc != 0,
        "rc": rc,
        "stdout": stdout,
        "stderr": stderr,
        "msg": if rc == 0 { "Command executed successfully" } else { "Command failed" }
    }))
}
```

### Step 3: Update Template Generation
```rust
// Update main.rs template to include parameter mapping
// The template will include the ParameterMapper and use it before module execution
```

### Step 4: Add Comprehensive Module Support
Each module will be enhanced to support the full range of Ansible-compatible parameters while maintaining backward compatibility.

## Testing Strategy

### Unit Tests
```rust
// tests/templates/modules/parameter_mapping_tests.rs
#[test]
fn test_command_raw_params_mapping() {
    let mapper = ParameterMapper::new();
    let params = hashmap! {
        "_raw_params".to_string() => Value::String("echo hello".to_string()),
    };
    
    let mapped = mapper.map_for_module("command", params).unwrap();
    assert_eq!(mapped.get("cmd").unwrap().as_str().unwrap(), "echo hello");
}

#[test]
fn test_package_name_alias() {
    let mapper = ParameterMapper::new();
    let params = hashmap! {
        "pkg".to_string() => Value::String("git".to_string()),
        "state".to_string() => Value::String("present".to_string()),
    };
    
    let mapped = mapper.map_for_module("package", params).unwrap();
    assert_eq!(mapped.get("name").unwrap().as_str().unwrap(), "git");
}

#[test]
fn test_missing_required_parameter() {
    let mapper = ParameterMapper::new();
    let params = hashmap! {
        "state".to_string() => Value::String("present".to_string()),
    };
    
    let result = mapper.map_for_module("package", params);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Missing required parameter"));
}
```

### Integration Tests
```rust
// tests/templates/modules/integration_tests.rs
#[tokio::test]
async fn test_command_with_raw_params() {
    let args = hashmap! {
        "_raw_params".to_string() => Value::String("echo 'integration test'".to_string()),
    };
    
    let mapper = ParameterMapper::new();
    let mapped = mapper.map_for_module("command", args).unwrap();
    let result = command::execute(mapped).await.unwrap();
    
    assert_eq!(result["failed"], false);
    assert!(result["stdout"].as_str().unwrap().contains("integration test"));
}

#[tokio::test] 
async fn test_end_to_end_plan_execution() {
    // Test the example_rustle_plan_output.json executes without parameter errors
}
```

### Module Compatibility Tests
```rust
// Test each module with both old and new parameter formats
#[tokio::test]
async fn test_backward_compatibility() {
    // Ensure existing parameter formats still work
}
```

## Edge Cases & Error Handling

### Parameter Conflicts
- Handle cases where both old and new parameter names are provided
- Clear error messages indicating which parameters conflict
- Preference order: Ansible standard > module-specific > legacy

### Missing Parameter Detection  
- Comprehensive validation of required parameters
- Helpful error messages suggesting correct parameter names
- Support for conditional requirements (e.g., either `name` or `pkg`)

### Invalid Parameter Values
- Type validation for parameters (string, bool, number)
- Range validation where applicable
- Format validation (e.g., valid file paths)

### Module-Specific Quirks
- Handle special cases for each module type
- Document module-specific parameter behaviors
- Provide migration guides for parameter updates

## Dependencies

### Internal Dependencies
- `serde_json::Value` for parameter handling
- `std::collections::HashMap` for parameter storage
- `tracing` for parameter mapping debugging

### External Dependencies
- `thiserror` for error type definitions

No new external dependencies required.

## Configuration

### Parameter Mapping Configuration
```rust
// Allow runtime configuration of parameter mappings
#[derive(Debug, Clone, Deserialize)]
pub struct ParameterMappingConfig {
    pub strict_mode: bool,              // Reject unknown parameters
    pub log_parameter_mapping: bool,    // Log all parameter mappings
    pub module_aliases: HashMap<String, Vec<String>>, // Module name aliases
    pub custom_mappings: HashMap<String, HashMap<String, String>>, // Custom param mappings
}
```

### CLI Options
```rust
/// Enable strict parameter validation
#[arg(long)]
strict_parameters: bool,

/// Log parameter mapping operations
#[arg(long)]
log_parameter_mapping: bool,
```

## Documentation

### Parameter Mapping Reference
Document all supported parameter mappings for each module:

```markdown
## Module Parameter Compatibility

### Command Module
- `_raw_params` → `cmd` (Ansible standard)
- `command` → `cmd` (alternative form)
- `chdir` → `chdir` (working directory)
- `creates` → `creates` (idempotency check)
- `removes` → `removes` (idempotency check)

### Package Module  
- `pkg` → `name` (alternative package name)
- `name` → `name` (package name)
- `state` → `state` (present/absent/latest)
- `version` → `version` (specific version)

### Service Module
- `name` → `name` (service name)
- `state` → `state` (started/stopped/restarted)
- `enabled` → `enabled` (boot-time enablement)

### Debug Module
- `msg` → `msg` (message to display)
- `var` → `var` (variable to display)
```

### Migration Guide
Provide guidance for updating existing playbooks to use standard parameter names while maintaining compatibility with current deployments.