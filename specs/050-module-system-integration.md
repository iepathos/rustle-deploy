# Spec 050: Module System Integration

## Feature Summary

Implement a comprehensive module system that enables dynamic loading, compilation, and static linking of custom task execution modules into deployment binaries. This system bridges the gap between rustle-plan's module requirements and the embedded runtime execution engine.

**Problem it solves**: The current implementation has placeholder module loading and cannot discover, compile, or integrate custom modules from execution plans, limiting functionality to basic built-in modules.

**High-level approach**: Create a module discovery system that can locate module sources, compile them into the deployment binary, and register them with the runtime execution engine for seamless task execution.

## Goals & Requirements

### Functional Requirements
- Discover modules required by execution plans
- Load module source code from various sources (filesystem, Git, HTTP, registry)
- Validate module compatibility and dependencies
- Compile custom modules into deployment binaries
- Register modules with runtime execution engine
- Support module versioning and conflict resolution
- Handle module dependency chains
- Provide module security validation and sandboxing
- Support both static and dynamic module loading
- Enable module hot-swapping for development

### Non-functional Requirements
- **Performance**: Module discovery and loading in <5 seconds for typical plans
- **Security**: Sandboxed module execution with permission controls
- **Compatibility**: Support standard Rust module patterns and Ansible module compatibility
- **Size**: Minimize binary size impact of unused modules
- **Reliability**: 99.9%+ module loading success rate for valid modules

### Success Criteria
- Successfully load and execute all required modules from execution plans
- Support custom module development workflow
- Provide secure module isolation and validation
- Enable efficient module compilation and caching
- Support module ecosystem development

## API/Interface Design

### Module Discovery and Loading

```rust
/// Module loader that discovers and loads modules for execution plans
pub struct ModuleLoader {
    module_cache: ModuleCache,
    source_resolvers: Vec<Box<dyn ModuleSourceResolver>>,
    validator: ModuleValidator,
    compiler: ModuleCompiler,
}

impl ModuleLoader {
    pub fn new() -> Self;
    
    pub async fn discover_modules(&self, execution_plan: &ExecutionPlan) -> Result<Vec<ModuleSpec>, ModuleError>;
    
    pub async fn load_module(&self, spec: &ModuleSpec) -> Result<LoadedModule, ModuleError>;
    
    pub async fn load_module_source(&self, source: &ModuleSource) -> Result<ModuleSourceCode, ModuleError>;
    
    pub fn validate_module(&self, module: &LoadedModule) -> Result<(), ValidationError>;
    
    pub async fn compile_module(&self, module: &LoadedModule, target: &str) -> Result<CompiledModule, CompileError>;
    
    pub fn resolve_dependencies(&self, modules: &[ModuleSpec]) -> Result<Vec<ModuleSpec>, DependencyError>;
    
    pub fn generate_module_registry(&self, modules: &[CompiledModule]) -> Result<String, GenerationError>;
}

/// Module specification from execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSpec {
    pub name: String,
    pub version: Option<String>,
    pub source: ModuleSource,
    pub checksum: Option<String>,
    pub dependencies: Vec<ModuleDependency>,
    pub requirements: ModuleRequirements,
    pub metadata: ModuleMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleSource {
    Builtin { name: String },
    File { path: String },
    Git { 
        repository: String, 
        reference: String,
        path: Option<String>,
    },
    Http { 
        url: String,
        headers: Option<HashMap<String, String>>,
    },
    Registry { 
        name: String, 
        version: String,
        registry_url: Option<String>,
    },
    Inline { source_code: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDependency {
    pub name: String,
    pub version_req: String,
    pub optional: bool,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleRequirements {
    pub rust_version: Option<String>,
    pub target_platforms: Vec<String>,
    pub required_capabilities: Vec<String>,
    pub security_level: SecurityLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityLevel {
    Trusted,    // Full system access
    Sandboxed,  // Limited filesystem/network access
    Isolated,   // No system access
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMetadata {
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub documentation: Option<String>,
    pub tags: Vec<String>,
}

/// Loaded module with source code and metadata
#[derive(Debug, Clone)]
pub struct LoadedModule {
    pub spec: ModuleSpec,
    pub source_code: ModuleSourceCode,
    pub manifest: ModuleManifest,
    pub resolved_dependencies: Vec<LoadedModule>,
}

#[derive(Debug, Clone)]
pub struct ModuleSourceCode {
    pub main_file: String,
    pub additional_files: HashMap<String, String>,
    pub cargo_toml: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleManifest {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub entry_point: String,
    pub required_args: Vec<String>,
    pub optional_args: Vec<String>,
    pub return_type: String,
    pub side_effects: Vec<String>,
    pub capabilities: Vec<String>,
}

/// Compiled module ready for embedding
#[derive(Debug, Clone)]
pub struct CompiledModule {
    pub spec: ModuleSpec,
    pub compiled_code: String,
    pub registration_code: String,
    pub static_data: Vec<u8>,
}
```

### Module Source Resolvers

```rust
/// Trait for resolving different module sources
pub trait ModuleSourceResolver: Send + Sync {
    fn can_resolve(&self, source: &ModuleSource) -> bool;
    
    async fn resolve(&self, source: &ModuleSource) -> Result<ModuleSourceCode, ResolveError>;
    
    fn cache_key(&self, source: &ModuleSource) -> String;
}

/// File system module resolver
pub struct FileSystemResolver {
    base_paths: Vec<PathBuf>,
}

impl ModuleSourceResolver for FileSystemResolver {
    fn can_resolve(&self, source: &ModuleSource) -> bool {
        matches!(source, ModuleSource::File { .. })
    }
    
    async fn resolve(&self, source: &ModuleSource) -> Result<ModuleSourceCode, ResolveError> {
        if let ModuleSource::File { path } = source {
            let full_path = self.resolve_path(path)?;
            let source_code = tokio::fs::read_to_string(&full_path).await?;
            
            // Look for additional files and Cargo.toml
            let directory = full_path.parent().unwrap();
            let additional_files = self.scan_directory(directory).await?;
            let cargo_toml = self.load_cargo_toml(directory).await?;
            
            Ok(ModuleSourceCode {
                main_file: source_code,
                additional_files,
                cargo_toml,
            })
        } else {
            Err(ResolveError::IncompatibleSource)
        }
    }
}

/// Git repository module resolver
pub struct GitResolver {
    cache_dir: PathBuf,
    git_client: GitClient,
}

impl ModuleSourceResolver for GitResolver {
    fn can_resolve(&self, source: &ModuleSource) -> bool {
        matches!(source, ModuleSource::Git { .. })
    }
    
    async fn resolve(&self, source: &ModuleSource) -> Result<ModuleSourceCode, ResolveError> {
        if let ModuleSource::Git { repository, reference, path } = source {
            let cache_key = self.cache_key(source);
            let cached_path = self.cache_dir.join(&cache_key);
            
            // Clone or update repository
            if !cached_path.exists() {
                self.git_client.clone_repository(repository, &cached_path).await?;
            } else {
                self.git_client.update_repository(&cached_path, reference).await?;
            }
            
            // Read module files
            let module_path = if let Some(path) = path {
                cached_path.join(path)
            } else {
                cached_path
            };
            
            self.load_from_directory(&module_path).await
        } else {
            Err(ResolveError::IncompatibleSource)
        }
    }
}

/// HTTP module resolver
pub struct HttpResolver {
    client: reqwest::Client,
    cache_dir: PathBuf,
}

/// Registry module resolver (for module registries)
pub struct RegistryResolver {
    client: reqwest::Client,
    registry_configs: HashMap<String, RegistryConfig>,
}

#[derive(Debug, Clone)]
pub struct RegistryConfig {
    pub base_url: String,
    pub auth_token: Option<String>,
    pub verify_signatures: bool,
}
```

### Module Validation and Security

```rust
/// Module validator for security and compatibility
pub struct ModuleValidator {
    security_policy: SecurityPolicy,
    compatibility_checker: CompatibilityChecker,
}

impl ModuleValidator {
    pub fn validate_module(&self, module: &LoadedModule) -> Result<ValidationResult, ValidationError> {
        let mut result = ValidationResult::new();
        
        // Security validation
        result.merge(self.validate_security(module)?);
        
        // Compatibility validation
        result.merge(self.validate_compatibility(module)?);
        
        // Dependency validation
        result.merge(self.validate_dependencies(module)?);
        
        // Code quality validation
        result.merge(self.validate_code_quality(module)?);
        
        Ok(result)
    }
    
    fn validate_security(&self, module: &LoadedModule) -> Result<ValidationResult, ValidationError> {
        let mut result = ValidationResult::new();
        
        // Check for dangerous operations
        let dangerous_patterns = [
            r"std::process::Command",
            r"std::fs::",
            r"std::net::",
            r"unsafe\s*{",
            r"#!\[.*unsafe.*\]",
        ];
        
        for pattern in &dangerous_patterns {
            if self.contains_pattern(&module.source_code.main_file, pattern) {
                match module.spec.requirements.security_level {
                    SecurityLevel::Isolated => {
                        result.add_error(format!("Isolated module contains dangerous pattern: {}", pattern));
                    }
                    SecurityLevel::Sandboxed => {
                        result.add_warning(format!("Sandboxed module contains restricted pattern: {}", pattern));
                    }
                    SecurityLevel::Trusted => {
                        // Allowed for trusted modules
                    }
                }
            }
        }
        
        Ok(result)
    }
    
    fn validate_compatibility(&self, module: &LoadedModule) -> Result<ValidationResult, ValidationError> {
        let mut result = ValidationResult::new();
        
        // Check Rust version compatibility
        if let Some(required_version) = &module.spec.requirements.rust_version {
            if !self.is_rust_version_compatible(required_version)? {
                result.add_error(format!("Incompatible Rust version: requires {}", required_version));
            }
        }
        
        // Check target platform compatibility
        for platform in &module.spec.requirements.target_platforms {
            if !self.is_platform_supported(platform) {
                result.add_warning(format!("Unsupported target platform: {}", platform));
            }
        }
        
        Ok(result)
    }
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub passed: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub info: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    pub allow_unsafe_code: bool,
    pub allow_network_access: bool,
    pub allow_filesystem_access: bool,
    pub allow_process_execution: bool,
    pub allowed_crates: Vec<String>,
    pub blocked_crates: Vec<String>,
}
```

### Module Compilation and Integration

```rust
/// Module compiler that generates code for embedding
pub struct ModuleCompiler {
    template_engine: TemplateEngine,
    code_generator: CodeGenerator,
}

impl ModuleCompiler {
    pub async fn compile_module(
        &self,
        module: &LoadedModule,
        target_triple: &str,
    ) -> Result<CompiledModule, CompileError> {
        // Generate wrapper code for the module
        let wrapper_code = self.generate_module_wrapper(module)?;
        
        // Generate registration code
        let registration_code = self.generate_registration_code(module)?;
        
        // Compile any static data
        let static_data = self.compile_static_data(module).await?;
        
        Ok(CompiledModule {
            spec: module.spec.clone(),
            compiled_code: wrapper_code,
            registration_code,
            static_data,
        })
    }
    
    fn generate_module_wrapper(&self, module: &LoadedModule) -> Result<String, CompileError> {
        let template = r#"
// Auto-generated wrapper for module: {{module_name}}
use crate::runtime::{Module, ModuleResult, ModuleError, ExecutionContext};
use std::collections::HashMap;
use serde_json::Value;

{{additional_imports}}

pub struct {{struct_name}};

impl Module for {{struct_name}} {
    fn name(&self) -> &str {
        "{{module_name}}"
    }
    
    fn description(&self) -> &str {
        "{{description}}"
    }
    
    fn required_args(&self) -> Vec<&str> {
        vec![{{required_args}}]
    }
    
    fn optional_args(&self) -> Vec<&str> {
        vec![{{optional_args}}]
    }
    
    fn execute(
        &self,
        args: &HashMap<String, Value>,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleError> {
        {{security_wrapper_start}}
        
        // Module implementation
        {{module_implementation}}
        
        {{security_wrapper_end}}
    }
}

{{module_source_code}}
"#;
        
        let context = TemplateContext {
            module_name: &module.spec.name,
            struct_name: &self.to_struct_name(&module.spec.name),
            description: module.manifest.description.as_deref().unwrap_or(""),
            required_args: &module.manifest.required_args.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(", "),
            optional_args: &module.manifest.optional_args.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(", "),
            module_implementation: &self.generate_module_implementation(module)?,
            module_source_code: &module.source_code.main_file,
            additional_imports: &self.generate_imports(module)?,
            security_wrapper_start: &self.generate_security_wrapper_start(module)?,
            security_wrapper_end: &self.generate_security_wrapper_end(module)?,
        };
        
        self.template_engine.render(template, &context)
    }
    
    fn generate_registration_code(&self, module: &LoadedModule) -> Result<String, CompileError> {
        let struct_name = self.to_struct_name(&module.spec.name);
        Ok(format!(
            "registry.register_module(\"{}\".to_string(), Box::new({}));",
            module.spec.name,
            struct_name
        ))
    }
    
    fn generate_security_wrapper_start(&self, module: &LoadedModule) -> Result<String, CompileError> {
        match module.spec.requirements.security_level {
            SecurityLevel::Isolated => Ok(r#"
                // Isolated module - no system access
                let _guard = SecurityGuard::new(SecurityLevel::Isolated);
            "#.to_string()),
            SecurityLevel::Sandboxed => Ok(r#"
                // Sandboxed module - limited system access
                let _guard = SecurityGuard::new(SecurityLevel::Sandboxed);
            "#.to_string()),
            SecurityLevel::Trusted => Ok(String::new()),
        }
    }
}

/// Security guard for module execution
pub struct SecurityGuard {
    level: SecurityLevel,
    original_permissions: SystemPermissions,
}

impl SecurityGuard {
    pub fn new(level: SecurityLevel) -> Self {
        let original_permissions = SystemPermissions::current();
        
        match level {
            SecurityLevel::Isolated => {
                // Disable all system access
                SecurityManager::disable_all_access();
            }
            SecurityLevel::Sandboxed => {
                // Limit system access
                SecurityManager::enable_sandboxed_access();
            }
            SecurityLevel::Trusted => {
                // No restrictions
            }
        }
        
        Self {
            level,
            original_permissions,
        }
    }
}

impl Drop for SecurityGuard {
    fn drop(&mut self) {
        // Restore original permissions
        SystemPermissions::restore(&self.original_permissions);
    }
}
```

### Module Cache

```rust
/// Cache for loaded and compiled modules
pub struct ModuleCache {
    cache_dir: PathBuf,
    memory_cache: HashMap<String, CachedModule>,
    ttl: Duration,
}

#[derive(Debug, Clone)]
struct CachedModule {
    module: LoadedModule,
    cached_at: Instant,
    access_count: usize,
}

impl ModuleCache {
    pub fn new(cache_dir: PathBuf, ttl: Duration) -> Self;
    
    pub async fn get(&mut self, spec: &ModuleSpec) -> Option<LoadedModule>;
    
    pub async fn store(&mut self, module: LoadedModule) -> Result<(), CacheError>;
    
    pub async fn invalidate(&mut self, spec: &ModuleSpec) -> Result<(), CacheError>;
    
    pub async fn cleanup_expired(&mut self) -> Result<(), CacheError>;
    
    pub fn get_cache_stats(&self) -> CacheStats;
    
    fn cache_key(&self, spec: &ModuleSpec) -> String {
        format!("{}:{}:{}", 
               spec.name, 
               spec.version.as_deref().unwrap_or("latest"),
               self.source_hash(&spec.source))
    }
}
```

## File and Package Structure

```
src/modules/
├── mod.rs                     # Module system exports
├── loader.rs                  # ModuleLoader implementation
├── resolver.rs                # Source resolvers
├── validator.rs               # Module validation
├── compiler.rs                # Module compilation
├── cache.rs                   # Module caching
├── security.rs                # Security enforcement
├── registry.rs                # Runtime module registry
├── template.rs                # Code generation templates
└── error.rs                   # Error types

src/modules/resolvers/
├── mod.rs
├── filesystem.rs              # File system resolver
├── git.rs                     # Git repository resolver
├── http.rs                    # HTTP resolver
└── registry.rs                # Module registry resolver

src/modules/builtin/
├── mod.rs
├── debug.rs                   # Debug module
├── command.rs                 # Command execution
├── shell.rs                   # Shell commands
├── copy.rs                    # File operations
├── template.rs                # Template processing
├── package.rs                 # Package management
├── service.rs                 # Service management
├── file.rs                    # File manipulation
└── setup.rs                   # Facts collection

templates/modules/
├── module_wrapper.rs.template
├── security_wrapper.rs.template
└── registration.rs.template

tests/modules/
├── loader_tests.rs
├── resolver_tests.rs
├── compiler_tests.rs
├── security_tests.rs
├── integration_tests.rs
└── fixtures/
    ├── test_modules/
    ├── malicious_modules/
    └── module_sources/
```

## Implementation Details

### Phase 1: Basic Module Loading
1. Implement ModuleLoader and basic source resolvers
2. Create filesystem and inline module support
3. Add basic module validation and compilation
4. Integrate with binary compilation pipeline

### Phase 2: Advanced Sources
1. Add Git and HTTP module resolvers
2. Implement module registry support
3. Create comprehensive validation system
4. Add security enforcement mechanisms

### Phase 3: Ecosystem Support
1. Implement module dependency resolution
2. Add module development tools
3. Create module testing framework
4. Add performance optimization

### Key Algorithms

**Module Discovery from Execution Plan**:
```rust
impl ModuleLoader {
    pub async fn discover_modules(&self, execution_plan: &ExecutionPlan) -> Result<Vec<ModuleSpec>, ModuleError> {
        let mut required_modules = HashSet::new();
        let mut module_specs = Vec::new();
        
        // Scan all tasks for required modules
        for play in &execution_plan.plays {
            for batch in &play.batches {
                for task in &batch.tasks {
                    required_modules.insert(task.module.clone());
                }
            }
            
            // Also check handlers
            for handler in &play.handlers {
                required_modules.insert(handler.module.clone());
            }
        }
        
        // Resolve module specifications
        for module_name in required_modules {
            let spec = self.resolve_module_spec(&module_name, &execution_plan.modules)?;
            module_specs.push(spec);
        }
        
        // Resolve dependencies
        let resolved_specs = self.resolve_dependencies(&module_specs)?;
        
        Ok(resolved_specs)
    }
    
    fn resolve_module_spec(
        &self,
        module_name: &str,
        defined_modules: &[ModuleSpec],
    ) -> Result<ModuleSpec, ModuleError> {
        // Check if module is explicitly defined in execution plan
        if let Some(spec) = defined_modules.iter().find(|m| m.name == module_name) {
            return Ok(spec.clone());
        }
        
        // Check if it's a built-in module
        if self.is_builtin_module(module_name) {
            return Ok(ModuleSpec {
                name: module_name.to_string(),
                version: Some("builtin".to_string()),
                source: ModuleSource::Builtin { name: module_name.to_string() },
                checksum: None,
                dependencies: vec![],
                requirements: ModuleRequirements {
                    rust_version: None,
                    target_platforms: vec!["*".to_string()],
                    required_capabilities: vec![],
                    security_level: SecurityLevel::Trusted,
                },
                metadata: ModuleMetadata {
                    description: Some(format!("Built-in {} module", module_name)),
                    author: Some("Rustle Core Team".to_string()),
                    license: Some("MIT".to_string()),
                    documentation: None,
                    tags: vec!["builtin".to_string()],
                },
            });
        }
        
        // Try to resolve from default sources
        Err(ModuleError::ModuleNotFound { 
            name: module_name.to_string(),
            searched_sources: vec!["builtin", "execution_plan"].into_iter().map(String::from).collect(),
        })
    }
}
```

**Module Compilation Integration**:
```rust
impl BinaryCompiler {
    pub async fn compile_binary_with_modules(
        &self,
        compilation: &BinaryCompilation,
        modules: &[CompiledModule],
    ) -> Result<CompiledBinary, CompileError> {
        // Generate project with embedded modules
        let temp_dir = tempfile::TempDir::new()?;
        let project_dir = temp_dir.path();
        
        // Generate base project structure
        self.generate_binary_project(project_dir, compilation).await?;
        
        // Add module implementations
        let modules_dir = project_dir.join("src").join("modules");
        tokio::fs::create_dir_all(&modules_dir).await?;
        
        // Write each module
        for (i, module) in modules.iter().enumerate() {
            let module_file = modules_dir.join(format!("module_{}.rs", i));
            tokio::fs::write(&module_file, &module.compiled_code).await?;
        }
        
        // Generate module registry
        let registry_code = self.generate_module_registry_code(modules)?;
        tokio::fs::write(
            modules_dir.join("registry.rs"),
            registry_code
        ).await?;
        
        // Update main.rs to include modules
        let main_rs = self.generate_main_with_modules(compilation, modules)?;
        tokio::fs::write(project_dir.join("src").join("main.rs"), main_rs).await?;
        
        // Compile the binary
        self.cross_compile(
            project_dir,
            &compilation.target_triple,
            &compilation.compilation_options,
        ).await
    }
    
    fn generate_module_registry_code(&self, modules: &[CompiledModule]) -> Result<String, CompileError> {
        let mut code = String::new();
        
        // Include all module files
        for (i, _module) in modules.iter().enumerate() {
            code.push_str(&format!("mod module_{};\n", i));
        }
        
        code.push_str("\n");
        code.push_str("use crate::runtime::ModuleRegistry;\n\n");
        code.push_str("pub fn register_all_modules(registry: &mut ModuleRegistry) {\n");
        
        // Add registration calls
        for (i, module) in modules.iter().enumerate() {
            code.push_str(&format!("    // Register {}\n", module.spec.name));
            code.push_str(&format!("    {};\n", module.registration_code.replace("registry", "registry")));
        }
        
        code.push_str("}\n");
        
        Ok(code)
    }
}
```

## Testing Strategy

### Unit Tests
- **Resolver Tests**: Each resolver type with various source formats
- **Validation Tests**: Security validation, compatibility checks
- **Compilation Tests**: Module wrapper generation, integration
- **Cache Tests**: Module caching and invalidation

### Integration Tests
- **End-to-end**: Complete module loading and execution workflow
- **Security Tests**: Module sandboxing and permission enforcement
- **Performance Tests**: Large module loading and compilation
- **Compatibility Tests**: Different module formats and versions

### Security Tests
```
tests/fixtures/modules/security/
├── trusted_module.rs          # Full system access module
├── sandboxed_module.rs        # Limited access module
├── isolated_module.rs         # No system access module
├── malicious_network.rs       # Network access attempt
├── malicious_filesystem.rs    # Filesystem access attempt
└── malicious_process.rs       # Process execution attempt
```

## Edge Cases & Error Handling

### Module Loading Edge Cases
- Network failures during Git/HTTP module loading
- Corrupted module sources and checksums
- Version conflicts between dependencies
- Circular module dependencies
- Large module sources exceeding size limits

### Security Edge Cases
- Module attempting to escape sandbox
- Malicious code injection attempts
- Resource exhaustion attacks
- Privilege escalation attempts
- Unsafe code in sandboxed modules

### Compilation Edge Cases
- Module compilation failures
- Target platform incompatibilities
- Memory limitations during compilation
- Conflicting module symbols
- Binary size limits exceeded

## Dependencies

### External Crates
```toml
[dependencies]
git2 = "0.18"               # Git repository operations
reqwest = { version = "0.11", features = ["stream"] }
tar = "0.4"                 # Archive extraction
flate2 = "1.0"              # Compression
sha2 = "0.10"               # Checksums
semver = "1.0"              # Version parsing
handlebars = "4.5"          # Template engine
walkdir = "2.4"             # Directory traversal
```

### Security Dependencies
```toml
[dependencies]
nix = { version = "0.27", optional = true }        # Unix permissions
winapi = { version = "0.3", optional = true }      # Windows permissions
libc = "0.2"                                       # System calls
```

## Configuration

### Module System Configuration
```toml
[modules]
cache_dir = "~/.rustle/modules"
cache_ttl_hours = 24
max_cache_size_mb = 1000
enable_git_modules = true
enable_http_modules = true
enable_registry_modules = true

[module_security]
default_security_level = "sandboxed"
allow_unsafe_modules = false
validate_checksums = true
block_network_access = true
block_filesystem_write = true

[module_sources]
default_registry = "https://modules.rustle.dev"
git_clone_timeout_secs = 300
http_download_timeout_secs = 60
max_module_size_mb = 10
```

## Documentation

### Module Development Guide
```rust
/// Example custom module implementation
use rustle_deploy::runtime::{Module, ModuleResult, ExecutionContext};

pub struct MyCustomModule;

impl Module for MyCustomModule {
    fn name(&self) -> &str { "my_custom" }
    
    fn description(&self) -> &str { "A custom task execution module" }
    
    fn required_args(&self) -> Vec<&str> { vec!["action"] }
    
    fn execute(&self, args: &HashMap<String, Value>, context: &ExecutionContext) -> Result<ModuleResult, ModuleError> {
        let action = args.get("action").unwrap().as_str().unwrap();
        
        match action {
            "hello" => Ok(ModuleResult {
                changed: false,
                failed: false,
                output: json!({"message": "Hello from custom module!"}),
                stdout: Some("Hello from custom module!".to_string()),
                stderr: None,
                message: Some("Hello executed successfully".to_string()),
            }),
            _ => Err(ModuleError::InvalidArg {
                arg: "action".to_string(),
                value: action.to_string(),
            }),
        }
    }
}
```

### Integration Examples
```rust
// Module loading in binary compilation
let module_loader = ModuleLoader::new();
let required_modules = module_loader.discover_modules(&execution_plan).await?;
let loaded_modules = module_loader.load_modules(&required_modules).await?;
let compiled_modules = module_loader.compile_modules(&loaded_modules, &target_triple).await?;

// Binary compilation with modules
let binary = compiler.compile_binary_with_modules(&compilation, &compiled_modules).await?;
```