# Spec 080: Binary Template Generation

## Feature Summary

Implement a comprehensive binary template generation system that creates optimized Rust source code for target binaries. This system takes execution plans and module requirements, generates complete Rust projects with embedded execution data, and prepares them for cross-compilation and deployment.

**Problem it solves**: The binary deployment pipeline needs to generate valid, optimized Rust source code that embeds execution plans, modules, and static files into self-contained binaries that can execute tasks locally on target hosts without network dependencies.

**High-level approach**: Create a template generation engine that produces complete Rust projects with embedded data, module implementations, runtime execution logic, and communication capabilities, optimized for each target architecture and execution plan.

## Goals & Requirements

### Functional Requirements
- Generate complete Rust source code from execution plans
- Embed execution plans, modules, and static files as compile-time data
- Create platform-specific runtime execution logic
- Generate Cargo.toml with appropriate dependencies
- Support multiple target architectures and platforms
- Embed controller communication capabilities
- Create optimized binary templates for performance
- Support incremental template generation and caching
- Generate template variants for different execution strategies
- Handle secrets and sensitive data embedding securely

### Non-functional Requirements
- **Performance**: Generate templates for 1000+ task plans in <500ms
- **Size**: Generated binaries under 50MB with compression
- **Security**: Safe embedding of secrets with encryption
- **Reliability**: Generated code compiles successfully 99.9%+ of the time
- **Optimization**: Templates produce optimized binaries with minimal overhead

### Success Criteria
- Generated templates compile successfully across all target platforms
- Embedded execution data is accessible and functional at runtime
- Binary size optimization reduces deployment overhead
- Template generation supports complex execution plans
- Incremental generation provides 90%+ time savings for similar plans

## API/Interface Design

### Core Template Generation

```rust
/// Binary template generator that creates Rust source code for deployment
pub struct BinaryTemplateGenerator {
    config: TemplateConfig,
    cache: TemplateCache,
    embedder: DataEmbedder,
    optimizer: TemplateOptimizer,
}

impl BinaryTemplateGenerator {
    pub fn new(config: TemplateConfig) -> Self;
    
    pub async fn generate_binary_template(
        &self,
        execution_plan: &RustlePlanOutput,
        binary_deployment: &BinaryDeploymentPlan,
        target_info: &TargetInfo,
    ) -> Result<GeneratedTemplate, TemplateError>;
    
    pub async fn generate_incremental_template(
        &self,
        base_template: &GeneratedTemplate,
        changes: &ExecutionPlanDiff,
    ) -> Result<GeneratedTemplate, TemplateError>;
    
    pub fn generate_cargo_toml(
        &self,
        dependencies: &[ModuleDependency],
        target_triple: &str,
    ) -> Result<String, TemplateError>;
    
    pub fn generate_main_rs(
        &self,
        execution_plan: &RustlePlanOutput,
        embedded_data: &EmbeddedData,
    ) -> Result<String, TemplateError>;
    
    pub fn generate_module_implementations(
        &self,
        modules: &[ModuleSpec],
        target_platform: &Platform,
    ) -> Result<HashMap<String, String>, TemplateError>;
    
    pub async fn optimize_template(
        &self,
        template: &GeneratedTemplate,
        optimization_level: OptimizationLevel,
    ) -> Result<GeneratedTemplate, TemplateError>;
}

/// Complete generated template ready for compilation
#[derive(Debug, Clone)]
pub struct GeneratedTemplate {
    pub template_id: String,
    pub source_files: HashMap<PathBuf, String>,
    pub embedded_data: EmbeddedData,
    pub cargo_toml: String,
    pub build_script: Option<String>,
    pub target_info: TargetInfo,
    pub compilation_flags: Vec<String>,
    pub estimated_binary_size: u64,
    pub cache_key: String,
}

#[derive(Debug, Clone)]
pub struct EmbeddedData {
    pub execution_plan: String,
    pub static_files: HashMap<String, Vec<u8>>,
    pub module_binaries: HashMap<String, Vec<u8>>,
    pub runtime_config: RuntimeConfig,
    pub secrets: EncryptedSecrets,
    pub facts_cache: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TargetInfo {
    pub target_triple: String,
    pub platform: Platform,
    pub architecture: String,
    pub os_family: String,
    pub libc: Option<String>,
    pub features: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub controller_endpoint: Option<String>,
    pub execution_timeout: Duration,
    pub report_interval: Duration,
    pub cleanup_on_completion: bool,
    pub log_level: String,
    pub heartbeat_interval: Duration,
    pub max_retries: u32,
}

#[derive(Debug, Clone)]
pub struct EncryptedSecrets {
    pub vault_data: HashMap<String, Vec<u8>>,
    pub encryption_key_id: String,
    pub decryption_method: String,
}
```

### Template Generation Strategies

```rust
pub struct TemplateOptimizer {
    strategies: Vec<Box<dyn OptimizationStrategy>>,
}

impl TemplateOptimizer {
    pub fn optimize_for_size(&self, template: &mut GeneratedTemplate) -> Result<(), TemplateError>;
    pub fn optimize_for_speed(&self, template: &mut GeneratedTemplate) -> Result<(), TemplateError>;
    pub fn optimize_for_memory(&self, template: &mut GeneratedTemplate) -> Result<(), TemplateError>;
}

#[async_trait]
pub trait OptimizationStrategy: Send + Sync {
    async fn apply(&self, template: &mut GeneratedTemplate) -> Result<(), TemplateError>;
    fn name(&self) -> &'static str;
    fn priority(&self) -> u8;
}

pub struct DeadCodeElimination;
pub struct StaticLinkingOptimizer;
pub struct CompressionOptimizer;
pub struct InliningOptimizer;
```

### Data Embedding System

```rust
pub struct DataEmbedder {
    encryptor: SecretEncryptor,
    compressor: DataCompressor,
}

impl DataEmbedder {
    pub fn embed_execution_plan(
        &self,
        plan: &RustlePlanOutput,
    ) -> Result<String, EmbedError>;
    
    pub fn embed_static_files(
        &self,
        files: &[StaticFile],
    ) -> Result<HashMap<String, String>, EmbedError>;
    
    pub fn embed_modules(
        &self,
        modules: &[CompiledModule],
    ) -> Result<HashMap<String, String>, EmbedError>;
    
    pub fn embed_secrets(
        &self,
        secrets: &[SecretSpec],
        target_key: &EncryptionKey,
    ) -> Result<EncryptedSecrets, EmbedError>;
    
    pub fn generate_embedded_data_accessors(
        &self,
        embedded_data: &EmbeddedData,
    ) -> Result<String, EmbedError>;
}

#[derive(Debug, Clone)]
pub struct StaticFile {
    pub source_path: PathBuf,
    pub embedded_path: String,
    pub content: Vec<u8>,
    pub permissions: u32,
    pub compression: CompressionType,
}

#[derive(Debug, Clone)]
pub enum CompressionType {
    None,
    Gzip,
    Lz4,
    Zstd,
}
```

## File and Package Structure

```
src/template/
├── mod.rs                     # Template generation module
├── generator.rs               # Main template generator
├── embedder.rs                # Data embedding system
├── optimizer.rs               # Template optimization
├── cache.rs                   # Template caching
├── templates/                 # Template files
│   ├── main_rs.template       # Main binary template
│   ├── cargo_toml.template    # Cargo.toml template
│   ├── runtime.template       # Runtime execution template
│   └── modules.template       # Module implementation template
└── platform/                  # Platform-specific templates
    ├── linux.rs              # Linux-specific generation
    ├── macos.rs               # macOS-specific generation
    └── windows.rs             # Windows-specific generation

src/compiler/
├── template.rs                # Template compilation (update existing)
└── embedding.rs               # Data embedding (update existing)

tests/template/
├── generator_tests.rs         # Template generation tests
├── embedder_tests.rs          # Data embedding tests
├── optimizer_tests.rs         # Optimization tests
├── integration_tests.rs       # End-to-end template tests
└── fixtures/
    ├── execution_plans/       # Test execution plans
    ├── expected_templates/    # Expected generated code
    └── static_files/          # Test static files
```

## Implementation Details

### Phase 1: Basic Template Generation
1. Implement core template generation infrastructure
2. Create basic main.rs and Cargo.toml templates
3. Add execution plan embedding
4. Create simple runtime execution logic

### Phase 2: Advanced Embedding
1. Implement static file embedding with compression
2. Add module implementation embedding
3. Create secret encryption and embedding
4. Add platform-specific template variants

### Phase 3: Optimization
1. Implement template optimization strategies
2. Add dead code elimination
3. Create size and performance optimizations
4. Add incremental template generation

### Phase 4: Caching and Performance
1. Implement template caching system
2. Add incremental generation for similar plans
3. Create template diff and merge capabilities
4. Add performance monitoring and metrics

### Key Template Algorithms

**Main Binary Template Generation**:
```rust
impl BinaryTemplateGenerator {
    pub fn generate_main_rs(
        &self,
        execution_plan: &RustlePlanOutput,
        embedded_data: &EmbeddedData,
    ) -> Result<String, TemplateError> {
        let template = format!(r#"
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::{{timeout, sleep}};
use serde_json::Value;
use anyhow::{{Result, Context}};

mod embedded_data {{
    use super::*;
    
    pub const EXECUTION_PLAN: &str = include_str!("execution_plan.json");
    pub const RUNTIME_CONFIG: &str = include_str!("runtime_config.json");
    
    pub fn get_static_files() -> HashMap<&'static str, &'static [u8]> {{
        let mut files = HashMap::new();
        {}
        files
    }}
}}

mod modules {{
    use super::*;
    {}
}}

mod runtime {{
    use super::*;
    
    pub struct LocalExecutor {{
        config: RuntimeConfig,
        module_registry: ModuleRegistry,
        facts: HashMap<String, Value>,
    }}
    
    impl LocalExecutor {{
        pub fn new(config: RuntimeConfig) -> Self {{
            Self {{
                config,
                module_registry: ModuleRegistry::new(),
                facts: HashMap::new(),
            }}
        }}
        
        pub async fn execute_plan(&mut self, plan: RustlePlanOutput) -> Result<ExecutionReport> {{
            let mut results = Vec::new();
            
            for play in &plan.plays {{
                let play_result = self.execute_play(play).await?;
                results.push(play_result);
            }}
            
            Ok(ExecutionReport {{
                success: results.iter().all(|r| r.success),
                results,
                execution_time: std::time::Instant::now().elapsed(),
            }})
        }}
        
        async fn execute_play(&mut self, play: &PlayPlan) -> Result<PlayResult> {{
            let mut task_results = Vec::new();
            
            for batch in &play.batches {{
                let batch_result = self.execute_batch(batch).await?;
                task_results.extend(batch_result.task_results);
            }}
            
            Ok(PlayResult {{
                play_id: play.play_id.clone(),
                success: task_results.iter().all(|r| !r.failed),
                task_results,
            }})
        }}
        
        async fn execute_batch(&mut self, batch: &TaskBatch) -> Result<BatchResult> {{
            let mut task_results = Vec::new();
            
            for task in &batch.tasks {{
                if let Some(timeout_duration) = self.config.execution_timeout {{
                    let result = timeout(timeout_duration, self.execute_task(task)).await??;
                    task_results.push(result);
                }} else {{
                    let result = self.execute_task(task).await?;
                    task_results.push(result);
                }}
            }}
            
            Ok(BatchResult {{
                batch_id: batch.batch_id.clone(),
                task_results,
            }})
        }}
        
        async fn execute_task(&mut self, task: &TaskPlan) -> Result<TaskResult> {{
            // Execute module with args
            let module_args = ModuleArgs {{
                args: task.args.clone(),
                special: SpecialParameters::default(),
            }};
            
            let context = ExecutionContext {{
                facts: self.facts.clone(),
                variables: HashMap::new(),
                host_info: HostInfo::detect(),
                working_directory: std::env::current_dir()?,
                environment: std::env::vars().collect(),
                check_mode: false,
                diff_mode: false,
                verbosity: 0,
            }};
            
            let result = self.module_registry.execute_module(
                &task.module,
                &module_args,
                &context,
            ).await?;
            
            Ok(TaskResult {{
                task_id: task.task_id.clone(),
                module_result: result,
                start_time: std::time::SystemTime::now(),
                duration: Duration::from_millis(0),
            }})
        }}
    }}
}}

#[tokio::main]
async fn main() -> Result<()> {{
    // Initialize logging
    tracing_subscriber::init();
    
    // Parse embedded execution plan
    let execution_plan: RustlePlanOutput = serde_json::from_str(embedded_data::EXECUTION_PLAN)
        .context("Failed to parse embedded execution plan")?;
    
    let runtime_config: RuntimeConfig = serde_json::from_str(embedded_data::RUNTIME_CONFIG)
        .context("Failed to parse runtime configuration")?;
    
    // Create executor
    let mut executor = runtime::LocalExecutor::new(runtime_config.clone());
    
    // Execute plan
    let start_time = std::time::Instant::now();
    let result = executor.execute_plan(execution_plan).await
        .context("Execution plan failed")?;
    
    let execution_time = start_time.elapsed();
    
    // Report results
    if let Some(controller_endpoint) = &runtime_config.controller_endpoint {{
        report_to_controller(&controller_endpoint, &result).await
            .context("Failed to report results to controller")?;
    }}
    
    // Cleanup if requested
    if runtime_config.cleanup_on_completion {{
        cleanup_runtime().await?;
    }}
    
    if result.success {{
        println!("Execution completed successfully in {{:?}}", execution_time);
        std::process::exit(0);
    }} else {{
        eprintln!("Execution failed after {{:?}}", execution_time);
        std::process::exit(1);
    }}
}}

async fn report_to_controller(endpoint: &str, result: &ExecutionReport) -> Result<()> {{
    // Send execution report to controller
    let client = reqwest::Client::new();
    let response = client
        .post(endpoint)
        .json(result)
        .send()
        .await?;
    
    if response.status().is_success() {{
        Ok(())
    }} else {{
        Err(anyhow::anyhow!("Controller reported error: {{}}", response.status()))
    }}
}}

async fn cleanup_runtime() -> Result<()> {{
    // Clean up temporary files and resources
    if let Ok(current_exe) = std::env::current_exe() {{
        tokio::fs::remove_file(current_exe).await.ok();
    }}
    Ok(())
}}
"#,
            self.generate_static_file_declarations(&embedded_data.static_files)?,
            self.generate_module_implementations(execution_plan)?,
        );
        
        Ok(template)
    }
    
    fn generate_static_file_declarations(
        &self,
        static_files: &HashMap<String, Vec<u8>>,
    ) -> Result<String, TemplateError> {
        let declarations = static_files.keys()
            .map(|path| format!(
                r#"files.insert("{}", include_bytes!("static_files/{}"));"#,
                path, path
            ))
            .collect::<Vec<_>>()
            .join("\n        ");
        
        Ok(declarations)
    }
}
```

**Cargo.toml Generation**:
```rust
impl BinaryTemplateGenerator {
    pub fn generate_cargo_toml(
        &self,
        dependencies: &[ModuleDependency],
        target_triple: &str,
    ) -> Result<String, TemplateError> {
        let mut cargo_toml = format!(r#"
[package]
name = "rustle-runner"
version = "1.0.0"
edition = "2021"

[[bin]]
name = "rustle-runner"
path = "main.rs"

[dependencies]
tokio = {{ version = "1", features = ["full"] }}
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
reqwest = {{ version = "0.11", features = ["json"] }}

# Module-specific dependencies
"#);
        
        // Add module dependencies
        for dep in dependencies {
            cargo_toml.push_str(&format!("{} = \"{}\"\n", dep.name, dep.version));
        }
        
        // Add target-specific optimizations
        cargo_toml.push_str(&format!(r#"
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true

[target.'{}']
rustflags = ["-C", "target-cpu=native"]
"#, target_triple));
        
        Ok(cargo_toml)
    }
}
```

## Testing Strategy

### Unit Tests
- **Template Generation**: Test template creation for various execution plans
- **Data Embedding**: Test embedding of execution plans, files, and secrets
- **Optimization**: Test template optimization strategies
- **Platform Support**: Test platform-specific template generation

### Integration Tests
- **Compilation**: Test that generated templates compile successfully
- **Execution**: Test that generated binaries execute correctly
- **Cross-Platform**: Test template generation for different targets
- **Performance**: Test template generation speed and output size

### Test Infrastructure
```
tests/fixtures/template/
├── execution_plans/
│   ├── simple_plan.json       # Basic execution plan
│   ├── complex_plan.json      # Multi-module plan
│   └── large_plan.json        # 1000+ task plan
├── expected_output/
│   ├── main_rs_simple.rs      # Expected main.rs for simple plan
│   ├── cargo_toml_linux.toml  # Expected Cargo.toml for Linux
│   └── cargo_toml_macos.toml  # Expected Cargo.toml for macOS
└── static_files/
    ├── config.yaml            # Test configuration file
    └── script.sh              # Test script file
```

## Edge Cases & Error Handling

### Template Generation Edge Cases
- Execution plans with no tasks
- Plans requiring unsupported modules
- Target platforms with missing toolchains
- Plans with circular dependencies
- Extremely large execution plans (>100MB)

### Embedding Edge Cases
- Binary files in static files
- Secrets requiring different encryption methods
- Files with special characters in paths
- Compressed data that doesn't compress well
- Memory constraints during embedding

### Compilation Edge Cases
- Generated code that doesn't compile
- Missing dependencies for target platform
- Binary size exceeding limits
- Optimization failures
- Cross-compilation toolchain issues

### Recovery Strategies
- Fallback to simpler templates when optimization fails
- Graceful degradation for unsupported features
- Alternative compilation strategies for problematic targets
- Template validation before compilation
- Incremental template generation for large plans

## Dependencies

### External Crates
```toml
[dependencies]
# Template processing
handlebars = "4.5"             # Template engine
minijinja = "1.0"              # Fast template engine
tera = "1.19"                  # Template engine alternative

# Code generation
proc-macro2 = "1.0"            # Token generation
quote = "1.0"                  # Code quotation
syn = { version = "2.0", features = ["full"] }  # Rust parsing

# Compression and encoding
flate2 = "1"                   # Gzip compression
lz4 = "1.24"                   # LZ4 compression
zstd = "0.12"                  # Zstandard compression
base64 = "0.21"                # Base64 encoding

# Encryption
aes-gcm = "0.10"               # AES-GCM encryption
chacha20poly1305 = "0.10"     # ChaCha20-Poly1305
ring = "0.16"                  # Cryptographic primitives
```

### Internal Dependencies
- `rustle_deploy::execution` - Execution plan types
- `rustle_deploy::types` - Core type definitions
- `rustle_deploy::modules` - Module system integration

## Configuration

### Template Configuration
```toml
[template]
# Template engine
engine = "handlebars"          # handlebars, minijinja, tera
cache_templates = true
template_dir = "templates/"

# Code generation
optimization_level = "release"
generate_docs = false
include_debug_info = false

# Embedding
compress_static_files = true
compression_algorithm = "zstd"  # none, gzip, lz4, zstd
embed_source_maps = false

# Security
encrypt_secrets = true
encryption_algorithm = "aes-gcm"
key_derivation = "pbkdf2"
```

### Environment Variables
- `RUSTLE_TEMPLATE_DIR`: Template files directory
- `RUSTLE_TEMPLATE_CACHE`: Enable template caching
- `RUSTLE_COMPRESSION_LEVEL`: Default compression level (1-9)
- `RUSTLE_ENCRYPTION_KEY`: Secret encryption key

## Documentation

### Template Documentation
```rust
/// Generate a complete Rust binary template from execution plan
/// 
/// # Arguments
/// * `execution_plan` - The execution plan to embed
/// * `binary_deployment` - Binary deployment configuration
/// * `target_info` - Target platform information
/// 
/// # Returns
/// * `Ok(GeneratedTemplate)` - Complete template ready for compilation
/// * `Err(TemplateError)` - Template generation failure
/// 
/// # Examples
/// ```rust
/// let generator = BinaryTemplateGenerator::new(config);
/// let template = generator.generate_binary_template(
///     &execution_plan,
///     &deployment_plan,
///     &target_info,
/// ).await?;
/// ```
```

### Usage Examples
```rust
// Basic template generation
let generator = BinaryTemplateGenerator::new(config);
let template = generator.generate_binary_template(
    &execution_plan,
    &binary_deployment,
    &target_info,
).await?;

// Write template to filesystem for compilation
let project_dir = PathBuf::from("/tmp/rustle-project");
std::fs::create_dir_all(&project_dir)?;

for (path, content) in &template.source_files {
    let file_path = project_dir.join(path);
    std::fs::write(file_path, content)?;
}

std::fs::write(project_dir.join("Cargo.toml"), &template.cargo_toml)?;
```

## Integration Points

### Compiler Integration
```rust
impl BinaryCompiler {
    pub async fn compile_from_template(
        &self,
        template: &GeneratedTemplate,
        output_path: &Path,
    ) -> Result<CompiledBinary, CompileError> {
        // Write template to temporary directory
        let temp_dir = self.create_temp_project(template)?;
        
        // Compile using cargo
        let binary = self.cross_compile_at_path(&temp_dir, &template.target_info).await?;
        
        // Clean up temporary directory
        std::fs::remove_dir_all(temp_dir)?;
        
        Ok(binary)
    }
}
```

### Deployment Manager Integration
Templates integrate seamlessly with the deployment pipeline:
1. **Generate**: Create template from execution plan
2. **Compile**: Cross-compile template to binary
3. **Deploy**: Deploy binary to target hosts
4. **Execute**: Binary runs embedded execution plan