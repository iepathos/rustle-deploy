# Spec 150: System Facts Gathering Module (setup)

## Feature Summary

Implement a comprehensive system facts gathering module equivalent to Ansible's `setup` module within the rustle-deploy binary execution system. This module will collect detailed information about the target system including hardware, operating system, network configuration, and environment details. Facts are essential for conditional logic in deployment scripts and provide the foundation for intelligent automation decisions.

**Important**: This module is independent of the `rustle-facts` CLI tool. Both collect system facts but are optimized for different contexts - this module for embedded binary execution, `rustle-facts` for CLI pipeline usage.

## Relationship to rustle-facts Tool

This `setup` module and the `rustle-facts` CLI tool are **completely independent implementations** that happen to collect similar system information:

| Aspect | setup Module (This Spec) | rustle-facts CLI Tool |
|--------|--------------------------|----------------------|
| **Purpose** | Single-host fact collection within binary execution | Multi-host fact collection for pipeline |
| **Context** | Embedded in compiled binaries | Standalone CLI tool |
| **Optimization** | Memory efficiency, embedded execution | Parallel collection, caching |
| **Output** | ModuleResult with ansible_facts | JSON to stdout |
| **Dependencies** | None outside rustle-deploy | None - fully independent |
| **Usage** | `- setup:` task in playbooks | `rustle-facts inventory.yml > facts.json` |

Both tools follow Unix philosophy by being independent and communicating through standardized data formats (JSON), not shared code.

## Goals & Requirements

### Functional Requirements
- **Comprehensive fact collection**: Gather OS, hardware, network, and user environment facts
- **Cross-platform support**: Linux, macOS, Windows, FreeBSD, OpenBSD, NetBSD
- **Fact caching**: Cache facts for performance optimization across multiple module executions
- **Selective gathering**: Allow filtering to collect only specific fact categories
- **JSON serialization**: All facts must be serializable for template use and storage

### Non-Functional Requirements
- Fast fact collection (< 2 seconds on typical systems)
- Minimal system resource usage during collection
- Reliable cross-platform detection algorithms
- Graceful handling of missing/inaccessible information
- Security-conscious fact collection (no sensitive data leakage)

### Success Criteria
- Complete fact collection on all supported platforms
- Performance benchmarks meet < 2 second target
- Facts integrate seamlessly with template and conditional modules
- Comprehensive test coverage across different system configurations
- Documentation with fact reference and usage examples

## API/Interface Design

### Setup Module Interface
```rust
#[async_trait]
impl ExecutionModule for SetupModule {
    async fn execute(&self, args: &ModuleArgs, context: &ExecutionContext) -> Result<ModuleResult, ModuleExecutionError>;
}

pub struct SetupArgs {
    pub gather_subset: Option<Vec<FactCategory>>,  // Specific fact categories to collect
    pub gather_timeout: Option<u64>,               // Timeout in seconds for fact collection
    pub filter: Option<Vec<String>>,               // Fact name filters (glob patterns)
    pub fact_path: Option<String>,                 // Path for custom fact scripts
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FactCategory {
    All,
    Hardware,
    Network,
    Virtual,
    Ohai,      // Chef Ohai-style facts
    Facter,    // Puppet Facter-style facts
    Platform,
    Distribution,
    Cmdline,
    Python,
    Env,
    Interfaces,
    Default,   // Essential facts only
}
```

### Fact Structure
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemFacts {
    // Platform and OS information
    pub ansible_system: String,               // "Linux", "Darwin", "Windows"
    pub ansible_os_family: String,            // "RedHat", "Debian", "Windows"
    pub ansible_distribution: String,         // "Ubuntu", "CentOS", "macOS"
    pub ansible_distribution_version: String, // "20.04", "8.2", "12.1"
    pub ansible_distribution_release: String, // "focal", "ootpa"
    pub ansible_architecture: String,         // "x86_64", "aarch64", "i386"
    pub ansible_machine: String,              // Hardware platform identifier
    pub ansible_kernel: String,               // Kernel version
    pub ansible_kernel_version: String,       // Full kernel version string
    
    // Hardware information
    pub ansible_processor: Vec<String>,       // CPU information
    pub ansible_processor_count: u32,         // Number of physical CPUs
    pub ansible_processor_cores: u32,         // Total CPU cores
    pub ansible_processor_threads_per_core: u32, // Threads per core
    pub ansible_processor_vcpus: u32,         // Total virtual CPUs
    pub ansible_memtotal_mb: u64,             // Total memory in MB
    pub ansible_memfree_mb: u64,              // Free memory in MB
    pub ansible_swaptotal_mb: u64,            // Total swap in MB
    pub ansible_swapfree_mb: u64,             // Free swap in MB
    
    // Network information
    pub ansible_all_ipv4_addresses: Vec<String>, // All IPv4 addresses
    pub ansible_all_ipv6_addresses: Vec<String>, // All IPv6 addresses
    pub ansible_default_ipv4: Option<DefaultInterface>, // Default IPv4 interface
    pub ansible_default_ipv6: Option<DefaultInterface>, // Default IPv6 interface
    pub ansible_hostname: String,             // Short hostname
    pub ansible_fqdn: String,                 // Fully qualified domain name
    pub ansible_domain: String,               // DNS domain
    pub ansible_interfaces: Vec<String>,      // List of network interfaces
    
    // Per-interface details (dynamic keys)
    #[serde(flatten)]
    pub interface_facts: HashMap<String, InterfaceFacts>,
    
    // Environment and user information
    pub ansible_user_id: String,              // Current user
    pub ansible_user_uid: u32,                // User ID
    pub ansible_user_gid: u32,                // Group ID
    pub ansible_user_gecos: String,           // User GECOS field
    pub ansible_user_dir: String,             // User home directory
    pub ansible_user_shell: String,           // User shell
    pub ansible_env: HashMap<String, String>, // Environment variables
    
    // System paths and configuration
    pub ansible_pkg_mgr: String,              // Package manager (apt, yum, brew, etc.)
    pub ansible_service_mgr: String,          // Service manager (systemd, launchd, etc.)
    pub ansible_python_version: String,       // Python version (if available)
    pub ansible_system_capabilities: Vec<String>, // System capabilities
    
    // Virtualization information
    pub ansible_virtualization_type: String, // "kvm", "vmware", "docker", "physical"
    pub ansible_virtualization_role: String, // "guest", "host", "NA"
    
    // Custom facts
    pub ansible_local: HashMap<String, serde_json::Value>, // Local custom facts
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultInterface {
    pub interface: String,
    pub address: String,
    pub gateway: String,
    pub network: String,
    pub netmask: String,
    pub broadcast: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceFacts {
    pub device: String,
    pub active: bool,
    pub type_: String,                  // "ether", "loopback", "bridge"
    pub macaddress: Option<String>,
    pub mtu: Option<u32>,
    pub ipv4: Option<InterfaceIPv4>,
    pub ipv6: Vec<InterfaceIPv6>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceIPv4 {
    pub address: String,
    pub netmask: String,
    pub network: String,
    pub broadcast: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceIPv6 {
    pub address: String,
    pub prefix: u8,
    pub scope: String,
}
```

## File and Package Structure

### Module Organization
```
src/modules/system/
├── mod.rs                     # System module declarations
├── setup.rs                   # Main setup/facts module
├── facts/
│   ├── mod.rs                 # Facts collection framework
│   ├── collector.rs           # Main facts collector
│   ├── cache.rs               # Facts caching system
│   ├── platform/
│   │   ├── mod.rs            # Platform-specific collectors
│   │   ├── linux.rs          # Linux fact collection
│   │   ├── macos.rs          # macOS fact collection
│   │   ├── windows.rs        # Windows fact collection
│   │   ├── freebsd.rs        # FreeBSD fact collection
│   │   └── unix_common.rs    # Common Unix functionality
│   ├── hardware/
│   │   ├── mod.rs            # Hardware fact collection
│   │   ├── cpu.rs            # CPU information
│   │   ├── memory.rs         # Memory information
│   │   └── storage.rs        # Storage information
│   ├── network/
│   │   ├── mod.rs            # Network fact collection
│   │   ├── interfaces.rs     # Network interface detection
│   │   ├── routing.rs        # Routing table analysis
│   │   └── dns.rs            # DNS configuration
│   └── custom/
│       ├── mod.rs            # Custom facts support
│       └── loader.rs         # Custom fact script loader
```

### Integration Points
- Update `src/modules/system/mod.rs` to include setup module
- Integrate with `ExecutionContext` for fact caching within rustle-deploy execution
- Add fact template helpers for use in template module
- **No dependencies on external rustle tools** - completely self-contained implementation

## Implementation Details

### 1. Cross-Platform Fact Collection
```rust
#[async_trait]
pub trait PlatformFactCollector: Send + Sync {
    async fn collect_platform_facts(&self) -> Result<HashMap<String, serde_json::Value>, FactError>;
    async fn collect_hardware_facts(&self) -> Result<HashMap<String, serde_json::Value>, FactError>;
    async fn collect_network_facts(&self) -> Result<HashMap<String, serde_json::Value>, FactError>;
    async fn collect_virtualization_facts(&self) -> Result<HashMap<String, serde_json::Value>, FactError>;
}

#[cfg(target_os = "linux")]
pub struct LinuxFactCollector;

#[cfg(target_os = "linux")]
#[async_trait]
impl PlatformFactCollector for LinuxFactCollector {
    async fn collect_platform_facts(&self) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();
        
        // Read /etc/os-release
        if let Ok(os_release) = tokio::fs::read_to_string("/etc/os-release").await {
            facts.extend(parse_os_release(&os_release)?);
        }
        
        // Read /proc/version
        if let Ok(version) = tokio::fs::read_to_string("/proc/version").await {
            facts.insert("ansible_kernel".to_string(), 
                        json!(extract_kernel_version(&version)));
        }
        
        Ok(facts)
    }
    
    async fn collect_hardware_facts(&self) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();
        
        // CPU information from /proc/cpuinfo
        if let Ok(cpuinfo) = tokio::fs::read_to_string("/proc/cpuinfo").await {
            facts.extend(parse_cpu_info(&cpuinfo)?);
        }
        
        // Memory information from /proc/meminfo
        if let Ok(meminfo) = tokio::fs::read_to_string("/proc/meminfo").await {
            facts.extend(parse_memory_info(&meminfo)?);
        }
        
        Ok(facts)
    }
}

#[cfg(target_os = "macos")]
pub struct MacOSFactCollector;

#[cfg(target_os = "macos")]
#[async_trait]
impl PlatformFactCollector for MacOSFactCollector {
    async fn collect_platform_facts(&self) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut facts = HashMap::new();
        
        // Use system_profiler and sysctl for macOS facts
        let sw_vers = Command::new("sw_vers").output().await?;
        facts.extend(parse_sw_vers(&sw_vers.stdout)?);
        
        Ok(facts)
    }
}
```

### 2. Fact Caching System
```rust
use std::time::{Duration, SystemTime};

pub struct FactCache {
    cache: Arc<RwLock<HashMap<String, CachedFacts>>>,
    default_ttl: Duration,
}

#[derive(Debug, Clone)]
struct CachedFacts {
    facts: SystemFacts,
    timestamp: SystemTime,
    ttl: Duration,
}

impl FactCache {
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            default_ttl,
        }
    }
    
    pub async fn get_facts(&self, host: &str) -> Option<SystemFacts> {
        let cache = self.cache.read().await;
        if let Some(cached) = cache.get(host) {
            if cached.timestamp.elapsed().unwrap_or(Duration::MAX) < cached.ttl {
                return Some(cached.facts.clone());
            }
        }
        None
    }
    
    pub async fn cache_facts(&self, host: &str, facts: SystemFacts, ttl: Option<Duration>) {
        let mut cache = self.cache.write().await;
        cache.insert(host.to_string(), CachedFacts {
            facts,
            timestamp: SystemTime::now(),
            ttl: ttl.unwrap_or(self.default_ttl),
        });
    }
}
```

### 3. Network Interface Detection
```rust
#[cfg(unix)]
async fn collect_network_interfaces() -> Result<Vec<InterfaceFacts>, FactError> {
    use std::process::Command;
    
    let mut interfaces = Vec::new();
    
    // Parse ifconfig output (cross-platform on Unix systems)
    let ifconfig = Command::new("ifconfig").output().await?;
    interfaces.extend(parse_ifconfig(&ifconfig.stdout)?);
    
    // On Linux, also read /proc/net/dev for additional info
    #[cfg(target_os = "linux")]
    if let Ok(net_dev) = tokio::fs::read_to_string("/proc/net/dev").await {
        enhance_interfaces_with_proc_net_dev(&mut interfaces, &net_dev)?;
    }
    
    Ok(interfaces)
}

#[cfg(windows)]
async fn collect_network_interfaces() -> Result<Vec<InterfaceFacts>, FactError> {
    // Use Windows APIs or PowerShell commands
    let output = Command::new("powershell")
        .arg("-Command")
        .arg("Get-NetAdapter | ConvertTo-Json")
        .output()
        .await?;
    
    parse_windows_network_adapters(&output.stdout)
}
```

### 4. Custom Facts Support
```rust
pub struct CustomFactsLoader {
    fact_paths: Vec<PathBuf>,
}

impl CustomFactsLoader {
    pub async fn load_custom_facts(&self) -> Result<HashMap<String, serde_json::Value>, FactError> {
        let mut custom_facts = HashMap::new();
        
        for path in &self.fact_paths {
            if path.is_dir() {
                custom_facts.extend(self.load_fact_directory(path).await?);
            } else if path.is_file() {
                custom_facts.extend(self.load_fact_file(path).await?);
            }
        }
        
        Ok(custom_facts)
    }
    
    async fn load_fact_file(&self, path: &Path) -> Result<HashMap<String, serde_json::Value>, FactError> {
        match path.extension().and_then(|s| s.to_str()) {
            Some("json") => {
                let content = tokio::fs::read_to_string(path).await?;
                Ok(serde_json::from_str(&content)?)
            }
            Some("yaml") | Some("yml") => {
                let content = tokio::fs::read_to_string(path).await?;
                Ok(serde_yaml::from_str(&content)?)
            }
            _ => {
                // Execute as script and capture JSON output
                let output = Command::new(path).output().await?;
                Ok(serde_json::from_slice(&output.stdout)?)
            }
        }
    }
}
```

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_fact_collection() {
        let module = SetupModule::new();
        let args = ModuleArgs::default();
        let context = ExecutionContext::new(false);
        
        let result = module.execute(&args, &context).await.unwrap();
        assert!(result.ansible_facts.contains_key("ansible_system"));
        assert!(result.ansible_facts.contains_key("ansible_architecture"));
    }
    
    #[test]
    fn test_os_release_parsing() {
        let os_release = r#"
NAME="Ubuntu"
VERSION="20.04.3 LTS (Focal Fossa)"
ID=ubuntu
ID_LIKE=debian
PRETTY_NAME="Ubuntu 20.04.3 LTS"
VERSION_ID="20.04"
        "#;
        
        let facts = parse_os_release(os_release).unwrap();
        assert_eq!(facts.get("ansible_distribution").unwrap(), "Ubuntu");
        assert_eq!(facts.get("ansible_distribution_version").unwrap(), "20.04");
    }
    
    #[test]
    fn test_cpu_info_parsing() {
        let cpuinfo = r#"
processor	: 0
vendor_id	: GenuineIntel
cpu family	: 6
model		: 142
model name	: Intel(R) Core(TM) i7-8565U CPU @ 1.80GHz
        "#;
        
        let facts = parse_cpu_info(cpuinfo).unwrap();
        assert_eq!(facts.get("ansible_processor_count").unwrap(), &json!(1));
    }
}
```

### Platform Integration Tests
```rust
// tests/facts/platform_tests.rs
#[cfg(target_os = "linux")]
#[tokio::test]
async fn test_linux_fact_collection() {
    let collector = LinuxFactCollector;
    let facts = collector.collect_platform_facts().await.unwrap();
    
    assert!(facts.contains_key("ansible_distribution"));
    assert!(facts.contains_key("ansible_kernel"));
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn test_macos_fact_collection() {
    let collector = MacOSFactCollector;
    let facts = collector.collect_platform_facts().await.unwrap();
    
    assert!(facts.contains_key("ansible_distribution"));
    assert_eq!(facts.get("ansible_distribution").unwrap(), "MacOSX");
}
```

### Performance Benchmarks
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_fact_collection(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("setup_module_execution", |b| {
        b.iter(|| {
            rt.block_on(async {
                let module = SetupModule::new();
                let args = ModuleArgs::default();
                let context = ExecutionContext::new(false);
                
                black_box(module.execute(&args, &context).await.unwrap());
            })
        })
    });
}

criterion_group!(benches, benchmark_fact_collection);
criterion_main!(benches);
```

## Edge Cases & Error Handling

### Platform Differences
- Handle missing system files gracefully (e.g., missing /proc on non-Linux)
- Adapt to different command output formats across platforms
- Handle permission issues when accessing system information

### Error Recovery
```rust
#[derive(thiserror::Error, Debug)]
pub enum FactError {
    #[error("System command failed: {command}")]
    CommandFailed { command: String },
    
    #[error("Failed to parse system information: {source}")]
    ParseError { source: String },
    
    #[error("Permission denied accessing: {path}")]
    PermissionDenied { path: String },
    
    #[error("Timeout collecting facts after {timeout}s")]
    Timeout { timeout: u64 },
    
    #[error("Network interface detection failed: {source}")]
    NetworkError { source: String },
    
    #[error("Custom fact loading failed: {path}")]
    CustomFactError { path: String },
}
```

### Graceful Degradation
- Continue fact collection even if some facts fail to collect
- Provide partial facts rather than complete failure
- Log warnings for missing facts but don't fail the module

## Dependencies

### System Commands
- **Linux**: `/proc` filesystem, `ifconfig`, `lscpu`, `free`
- **macOS**: `system_profiler`, `sysctl`, `ifconfig`, `sw_vers`
- **Windows**: PowerShell cmdlets, WMI queries, registry access
- **FreeBSD/OpenBSD**: `sysctl`, `ifconfig`, `/proc` (if mounted)

### External Crates
- `hostname = "0.4"` (already available) - Hostname detection
- `serde_json = "1"` (already available) - JSON serialization
- `serde_yaml = "0.9"` (already available) - YAML custom facts
- `tokio` (already available) - Async file operations
- `regex = "1.10"` (already available) - Text parsing

### Internal Dependencies
- `crate::modules::interface` - Module interface traits within rustle-deploy
- `crate::execution::context` - Execution context with fact caching within rustle-deploy
- `crate::types::platform` - Platform detection within rustle-deploy
- **No external rustle tool dependencies** - self-contained implementation

## Configuration

### Module Configuration
```rust
pub struct SetupModuleConfig {
    pub fact_cache_ttl: Duration,           // Default: 1 hour
    pub collection_timeout: Duration,       // Default: 30 seconds
    pub custom_fact_paths: Vec<PathBuf>,    // Default: ["/etc/ansible/facts.d"]
    pub gather_network_resources: bool,     // Default: true
    pub gather_hardware_facts: bool,        // Default: true
    pub max_custom_fact_size: usize,        // Default: 1MB
}
```

### Environment Variables
- `RUSTLE_FACTS_CACHE_TTL` - Fact cache time-to-live in seconds
- `RUSTLE_FACTS_TIMEOUT` - Fact collection timeout in seconds
- `RUSTLE_CUSTOM_FACTS_PATH` - Custom facts directory path

## Documentation

### Fact Reference Documentation
Complete documentation of all collected facts:
- Platform-specific fact availability
- Fact naming conventions and formats
- Example values for each fact
- Conditional logic usage examples

### Usage Examples
```yaml
# Basic fact collection
- name: Gather system facts
  setup:

# Selective fact collection
- name: Gather only hardware facts
  setup:
    gather_subset:
      - hardware

# Conditional tasks based on facts
- name: Install package manager specific package
  package:
    name: "{{ package_name }}"
  when: ansible_pkg_mgr == "apt"

# Use facts in templates
- name: Configure application
  template:
    src: app.conf.j2
    dest: /etc/app/app.conf
    variables:
      hostname: "{{ ansible_hostname }}"
      cpu_count: "{{ ansible_processor_vcpus }}"
      memory_mb: "{{ ansible_memtotal_mb }}"
```

This specification provides comprehensive system fact gathering capabilities within the rustle-deploy binary execution system that will enable intelligent deployment automation based on target system characteristics. The implementation is completely independent of the `rustle-facts` CLI tool, following Unix philosophy of tool independence while serving the specific needs of embedded binary execution.