# Spec 030: Inventory Processing and Host Management

## Feature Summary

Implement inventory processing and host management to convert parsed inventory data from rustle-plan into deployment targets. This component processes structured inventory data and extracts host information, connection details, and target architecture information needed for binary deployment.

**Problem it solves**: rustle-deploy currently has placeholder inventory processing that returns hardcoded data, preventing real deployment to multiple hosts with proper connection configurations.

**High-level approach**: Create a flexible inventory processor that converts structured inventory data from rustle-plan, extracts host metadata, determines target architectures, and generates properly configured deployment targets.

**Note**: This spec focuses on inventory data processing, not raw file parsing. Raw inventory file parsing (YAML, JSON, INI) is handled by rustle-parse in the modular architecture pipeline.

## Goals & Requirements

### Functional Requirements
- Process structured inventory data from rustle-plan output
- Extract host connection information (SSH, WinRM, local)
- Determine target architecture for each host
- Support host groups and nested group structures
- Handle host variables and group variables
- Generate deployment targets from inventory data
- Validate host connectivity and requirements
- Convert between inventory formats for compatibility
- Handle inventory variable inheritance

### Non-functional Requirements
- **Performance**: Process inventory with 1000+ hosts in <200ms
- **Reliability**: 99.9%+ processing success rate for valid inventory data
- **Compatibility**: Support inventory data from rustle-plan output
- **Memory**: Efficient processing for inventories up to 50MB
- **Security**: Secure handling of connection credentials

### Success Criteria
- Successfully process inventory data from rustle-plan
- Correctly extract host connection information
- Determine target architectures automatically
- Generate valid deployment targets
- Support large-scale inventory processing

## API/Interface Design

### Core Data Structures

```rust
/// Parsed inventory with hosts and groups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedInventory {
    pub hosts: HashMap<String, InventoryHost>,
    pub groups: HashMap<String, InventoryGroup>,
    pub global_vars: HashMap<String, serde_json::Value>,
    pub metadata: InventoryMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryHost {
    pub name: String,
    pub address: Option<String>,
    pub connection: ConnectionConfig,
    pub variables: HashMap<String, serde_json::Value>,
    pub groups: Vec<String>,
    pub target_triple: Option<String>,
    pub architecture: Option<String>,
    pub operating_system: Option<String>,
    pub platform: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryGroup {
    pub name: String,
    pub hosts: Vec<String>,
    pub children: Vec<String>,
    pub variables: HashMap<String, serde_json::Value>,
    pub parent_groups: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryMetadata {
    pub format: InventoryFormat,
    pub source: String,
    pub parsed_at: DateTime<Utc>,
    pub host_count: usize,
    pub group_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InventoryFormat {
    Yaml,
    Json,
    Ini,
    Dynamic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub method: ConnectionMethod,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub private_key_file: Option<String>,
    pub timeout: Option<Duration>,
    pub ssh_args: Option<String>,
    pub winrm_transport: Option<WinRmTransport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionMethod {
    Ssh,
    WinRm,
    Local,
    Podman,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WinRmTransport {
    Http,
    Https,
    Kerberos,
    Ntlm,
}

/// Architecture detection and target mapping
#[derive(Debug, Clone)]
pub struct ArchitectureDetector {
    pub target_mappings: HashMap<String, String>,
    pub platform_mappings: HashMap<String, String>,
}

impl ArchitectureDetector {
    pub fn detect_target_triple(&self, host: &InventoryHost) -> Option<String>;
    pub fn map_platform_to_triple(&self, platform: &str, arch: &str) -> Option<String>;
    pub fn probe_host_architecture(&self, host: &InventoryHost) -> Result<String, ProbeError>;
}
```

### Processor API

```rust
pub struct InventoryProcessor {
    detector: ArchitectureDetector,
    validators: Vec<Box<dyn InventoryValidator>>,
}

impl InventoryProcessor {
    pub fn new() -> Self;
    
    pub fn process_from_plan(&self, plan_output: &serde_json::Value) -> Result<ParsedInventory, InventoryError>;
    
    pub fn process_inventory_data(&self, inventory_data: &ParsedInventory) -> Result<ParsedInventory, InventoryError>;
    
    pub fn validate(&self, inventory: &ParsedInventory) -> Result<(), ValidationError>;
    
    pub fn resolve_variables(&self, inventory: &mut ParsedInventory) -> Result<(), VariableError>;
    
    pub fn detect_architectures(&self, inventory: &mut ParsedInventory) -> Result<(), DetectionError>;
    
    pub fn to_deployment_targets(&self, inventory: &ParsedInventory) -> Result<Vec<DeploymentTarget>, ConversionError>;
    
    pub fn probe_host_info(&self, host: &InventoryHost) -> Result<HostInfo, ProbeError>;
}

/// JSON inventory processor (rustle-plan output)
pub struct JsonInventoryProcessor;

impl JsonInventoryProcessor {
    pub fn process_from_plan_output(
        &self, 
        plan_output: &serde_json::Value
    ) -> Result<ParsedInventory, InventoryError>;
    
    pub fn extract_inventory_section(
        &self,
        plan_output: &serde_json::Value
    ) -> Result<serde_json::Value, InventoryError>;
    
    pub fn process_hosts_data(
        &self, 
        hosts_data: &serde_json::Value
    ) -> Result<HashMap<String, InventoryHost>, InventoryError>;
    
    pub fn process_groups_data(
        &self, 
        groups_data: &serde_json::Value
    ) -> Result<HashMap<String, InventoryGroup>, InventoryError>;
}

/// Host information detection
#[derive(Debug, Clone)]
pub struct HostInfo {
    pub architecture: String,
    pub operating_system: String,
    pub platform: String,
    pub kernel_version: String,
    pub target_triple: String,
    pub capabilities: Vec<String>,
}

pub trait InventoryValidator {
    fn validate(&self, inventory: &ParsedInventory) -> Result<(), ValidationError>;
}

pub struct ConnectivityValidator;
pub struct ArchitectureValidator;
pub struct VariableValidator;
```

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum InventoryError {
    #[error("Invalid YAML format: {reason}")]
    InvalidYaml { reason: String },
    
    #[error("Invalid JSON format: {reason}")]
    InvalidJson { reason: String },
    
    #[error("Invalid INI format: {reason}")]
    InvalidIni { reason: String },
    
    #[error("Unsupported inventory format")]
    UnsupportedFormat,
    
    #[error("File not found: {path}")]
    FileNotFound { path: String },
    
    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },
    
    #[error("Dynamic inventory script failed: {script}")]
    DynamicScriptFailed { script: String },
    
    #[error("Variable resolution failed: {variable}")]
    VariableResolution { variable: String },
    
    #[error("Host connectivity check failed: {host}")]
    ConnectivityFailed { host: String },
    
    #[error("Architecture detection failed: {host}")]
    ArchitectureDetectionFailed { host: String },
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Duplicate host name: {host}")]
    DuplicateHost { host: String },
    
    #[error("Circular group dependency: {cycle:?}")]
    CircularGroupDependency { cycle: Vec<String> },
    
    #[error("Missing group: {group}")]
    MissingGroup { group: String },
    
    #[error("Invalid connection configuration for host: {host}")]
    InvalidConnection { host: String },
    
    #[error("Unreachable host: {host}")]
    UnreachableHost { host: String },
}

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("Connection failed: {host}")]
    ConnectionFailed { host: String },
    
    #[error("Authentication failed: {host}")]
    AuthenticationFailed { host: String },
    
    #[error("Command execution failed: {command}")]
    CommandFailed { command: String },
    
    #[error("Timeout exceeded: {timeout_secs}s")]
    Timeout { timeout_secs: u64 },
}
```

## File and Package Structure

```
src/inventory/
├── mod.rs                     # Module exports
├── processor.rs               # Main InventoryProcessor
├── plan_processor.rs          # Process rustle-plan output
├── validator.rs               # Inventory validation
├── detector.rs                # Architecture detection
├── host_info.rs               # Host information probing
├── variables.rs               # Variable resolution
└── error.rs                   # Error types

src/types/
├── inventory.rs               # Inventory data structures

tests/inventory/
├── processor_tests.rs
├── plan_processor_tests.rs
├── detection_tests.rs
├── validation_tests.rs
└── fixtures/
    ├── rustle_plan_outputs/
    │   ├── simple_plan.json
    │   ├── complex_plan.json
    │   └── multi_host_plan.json
    ├── inventory_data/
    │   ├── simple_inventory.json
    │   └── complex_inventory.json
    └── invalid_data/
```

## Implementation Details

### Phase 1: Basic Processing
1. Implement rustle-plan output processor
2. Create basic host and group data structures
3. Add deployment target generation
4. Integrate with existing deployment workflow

### Phase 2: Advanced Features
1. Implement variable resolution and inheritance
2. Create architecture detection and mapping
3. Add host connectivity validation
4. Support complex inventory structures

### Phase 3: Optimization
1. Add inventory data caching
2. Implement parallel processing
3. Create performance optimizations
4. Add comprehensive validation

### Key Algorithms

**Variable Resolution with Inheritance**:
```rust
impl InventoryProcessor {
    fn resolve_variables(&self, inventory: &mut ParsedInventory) -> Result<(), VariableError> {
        for host_name in inventory.hosts.keys().cloned().collect::<Vec<_>>() {
            let mut resolved_vars = inventory.global_vars.clone();
            
            // Collect variables from all groups (in order)
            let host = inventory.hosts.get(&host_name).unwrap();
            for group_name in &host.groups {
                if let Some(group) = inventory.groups.get(group_name) {
                    // Recursively resolve parent group variables
                    self.resolve_group_variables(group, &inventory.groups, &mut resolved_vars)?;
                    
                    // Apply group variables
                    for (key, value) in &group.variables {
                        resolved_vars.insert(key.clone(), value.clone());
                    }
                }
            }
            
            // Apply host-specific variables (highest priority)
            for (key, value) in &host.variables {
                resolved_vars.insert(key.clone(), value.clone());
            }
            
            // Update host with resolved variables
            inventory.hosts.get_mut(&host_name).unwrap().variables = resolved_vars;
        }
        
        Ok(())
    }
    
    fn resolve_group_variables(
        &self,
        group: &InventoryGroup,
        all_groups: &HashMap<String, InventoryGroup>,
        vars: &mut HashMap<String, serde_json::Value>,
    ) -> Result<(), VariableError> {
        // Recursively resolve parent group variables first
        for parent_name in &group.parent_groups {
            if let Some(parent_group) = all_groups.get(parent_name) {
                self.resolve_group_variables(parent_group, all_groups, vars)?;
                for (key, value) in &parent_group.variables {
                    vars.insert(key.clone(), value.clone());
                }
            }
        }
        
        Ok(())
    }
}
```

**Architecture Detection**:
```rust
impl ArchitectureDetector {
    pub fn detect_target_triple(&self, host: &InventoryHost) -> Option<String> {
        // Check explicit target_triple variable
        if let Some(triple) = &host.target_triple {
            return Some(triple.clone());
        }
        
        // Check architecture and platform variables
        if let (Some(arch), Some(platform)) = (
            host.variables.get("ansible_architecture"),
            host.variables.get("ansible_os_family")
        ) {
            let arch_str = arch.as_str()?;
            let platform_str = platform.as_str()?;
            return self.map_platform_to_triple(platform_str, arch_str);
        }
        
        // Fallback to connection-based detection
        match host.connection.method {
            ConnectionMethod::Local => Some(self.detect_local_triple()),
            ConnectionMethod::Ssh => self.probe_ssh_architecture(host),
            ConnectionMethod::WinRm => Some("x86_64-pc-windows-msvc".to_string()),
            _ => None,
        }
    }
    
    fn map_platform_to_triple(&self, platform: &str, arch: &str) -> Option<String> {
        match (platform.to_lowercase().as_str(), arch) {
            ("debian" | "ubuntu" | "redhat" | "centos" | "fedora", "x86_64") => {
                Some("x86_64-unknown-linux-gnu".to_string())
            }
            ("debian" | "ubuntu" | "redhat" | "centos" | "fedora", "aarch64") => {
                Some("aarch64-unknown-linux-gnu".to_string())
            }
            ("darwin", "x86_64") => {
                Some("x86_64-apple-darwin".to_string())
            }
            ("darwin", "arm64") => {
                Some("aarch64-apple-darwin".to_string())
            }
            ("windows", "amd64" | "x86_64") => {
                Some("x86_64-pc-windows-msvc".to_string())
            }
            _ => None,
        }
    }
}
```

**Deployment Target Generation**:
```rust
impl InventoryProcessor {
    pub fn to_deployment_targets(&self, inventory: &ParsedInventory) -> Result<Vec<DeploymentTarget>, ConversionError> {
        let mut targets = Vec::new();
        
        for (host_name, host) in &inventory.hosts {
            let target_triple = self.detector.detect_target_triple(host)
                .unwrap_or_else(|| "x86_64-unknown-linux-gnu".to_string());
            
            let deployment_method = match host.connection.method {
                ConnectionMethod::Ssh => DeploymentMethod::Ssh,
                ConnectionMethod::WinRm => DeploymentMethod::Custom {
                    command: format!("winrm copy {{binary_path}} {}/rustle-runner.exe", 
                                   host.connection.host.as_ref().unwrap_or(host_name))
                },
                ConnectionMethod::Local => DeploymentMethod::Scp,
                _ => DeploymentMethod::Ssh,
            };
            
            let target_path = match target_triple.contains("windows") {
                true => "C:\\temp\\rustle-runner.exe".to_string(),
                false => "/tmp/rustle-runner".to_string(),
            };
            
            targets.push(DeploymentTarget {
                host: host.connection.host.clone().unwrap_or_else(|| host_name.clone()),
                target_path,
                binary_compilation_id: format!("rustle-{}", target_triple),
                deployment_method,
                status: DeploymentStatus::Pending,
                deployed_at: None,
                version: "1.0.0".to_string(),
            });
        }
        
        Ok(targets)
    }
}
```

## Testing Strategy

### Unit Tests
- **Parser Tests**: Format-specific parsing for YAML, JSON, INI
- **Validation Tests**: Host validation, group validation, connectivity checks
- **Detection Tests**: Architecture detection, target triple mapping
- **Variable Tests**: Variable resolution, inheritance, scoping

### Integration Tests
- **End-to-end**: Complete inventory processing workflow
- **Format Compatibility**: Cross-format consistency checks
- **Large Scale**: Performance testing with 1000+ hosts
- **Real World**: Integration with actual Ansible inventories

### Test Data
```
tests/fixtures/inventories/
├── formats/
│   ├── simple.yml              # Basic YAML inventory
│   ├── simple.json             # Basic JSON inventory
│   ├── simple.ini              # Basic INI inventory
│   └── rustle_plan_output.json # Real rustle-plan output
├── complex/
│   ├── multi_group.yml         # Multiple groups and inheritance
│   ├── variables.yml           # Variable resolution testing
│   ├── mixed_platforms.yml     # Different OS/architectures
│   └── large_inventory.yml     # Performance testing (1000+ hosts)
├── ansible/
│   ├── production.yml          # Real Ansible production inventory
│   ├── development.ini         # Ansible INI format
│   └── dynamic_aws.py          # Dynamic inventory script
└── invalid/
    ├── circular_groups.yml     # Circular group dependencies
    ├── duplicate_hosts.yml     # Duplicate host names
    ├── missing_groups.yml      # Reference to non-existent groups
    └── malformed.json          # Syntax errors
```

## Edge Cases & Error Handling

### Parsing Edge Cases
- Mixed case host names and group names
- Unicode characters in host names
- Large inventories with deep group nesting
- Dynamic inventory script failures
- Network timeouts during URL-based inventory loading

### Variable Resolution Edge Cases
- Circular group dependencies
- Variable name conflicts between groups
- Complex variable types (lists, objects)
- Template variables within inventory variables

### Architecture Detection Edge Cases
- Unknown platforms or architectures
- Conflicting architecture information
- Network issues during host probing
- Unsupported target platforms

### Deployment Target Edge Cases
- Hosts with multiple network interfaces
- Dynamic IP addresses
- Port conflicts and firewall restrictions
- Authentication failures

## Dependencies

### External Crates
```toml
[dependencies]
serde_yaml = "0.9"
serde_ini = "0.2"
indexmap = "2.0"      # Ordered maps for consistent processing
url = "2.4"           # URL parsing for dynamic inventories
tokio-process = "0.2" # Process execution for dynamic scripts
ssh2 = "0.9"          # SSH connectivity testing
```

### Internal Dependencies
- `rustle_deploy::types` - DeploymentTarget and related types
- `rustle_deploy::deploy` - Integration with deployment manager
- `rustle_deploy::error` - Error handling patterns

## Configuration

### Inventory Configuration
```toml
[inventory]
default_format = "auto"
validate_connectivity = true
probe_architectures = false
timeout_secs = 30
max_hosts = 10000

[detection]
enable_probing = false
cache_results = true
cache_ttl_secs = 3600
fallback_target = "x86_64-unknown-linux-gnu"

[connection]
ssh_timeout_secs = 30
winrm_timeout_secs = 60
max_concurrent_probes = 10
```

### Environment Variables
- `RUSTLE_INVENTORY_PATH`: Default inventory file location
- `RUSTLE_PROBE_HOSTS`: Enable/disable host architecture probing
- `RUSTLE_INVENTORY_TIMEOUT`: Inventory processing timeout
- `RUSTLE_SSH_KEY_PATH`: Default SSH private key location

## Documentation

### API Documentation
```rust
/// Parse inventory from multiple formats with automatic detection
/// 
/// # Arguments
/// * `content` - Inventory content (YAML, JSON, or INI)
/// * `format` - Expected format or Auto for detection
/// 
/// # Returns
/// * `Ok(ParsedInventory)` - Successfully parsed inventory
/// * `Err(InventoryError)` - Parsing or validation failure
/// 
/// # Examples
/// ```rust
/// let content = std::fs::read_to_string("inventory.yml")?;
/// let parser = InventoryParser::new();
/// let inventory = parser.parse(&content, InventoryFormat::Auto)?;
/// let targets = parser.to_deployment_targets(&inventory)?;
/// ```
```

### Integration Examples
```rust
// Process from rustle-plan output
let plan_output: serde_json::Value = serde_json::from_str(&rustle_plan_json)?;
let processor = InventoryProcessor::new();
let inventory = processor.process_from_plan(&plan_output)?;

// Process pre-parsed inventory data
let inventory_data = load_inventory_data_from_cache()?;
let processed_inventory = processor.process_inventory_data(&inventory_data)?;

// Convert to deployment targets
let targets = processor.to_deployment_targets(&processed_inventory)?;

// Integration with deployment manager
let deployment_plan = manager.create_deployment_plan_from_inventory(
    &execution_plan,
    &processed_inventory,
).await?;
```