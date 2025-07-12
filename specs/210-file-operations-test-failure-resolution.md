# Spec 210: File Operations Test Failure Resolution

## Feature Summary

Resolve the 25 failing tests in the comprehensive file operations testing suite implemented in spec 200. The test failures fall into four main categories: stat module missing functionality, template engine syntax incompatibility, copy module missing features, and workflow integration issues. This specification addresses each category systematically to achieve 100% test pass rate while maintaining backward compatibility with existing functionality.

**Architecture Note**: This specification focuses on fixing implementation gaps and incompatibilities without changing the core architecture, ensuring the comprehensive test suite validates actual working functionality.

## Goals & Requirements

### Functional Requirements
- **Stat module enhancement**: Implement missing `ansible_facts` return format, checksum calculation, and file attribute gathering
- **Template engine compatibility**: Fix Handlebars template syntax to support expected filtering and conditional operations
- **Copy module completion**: Add directory copying, attribute preservation, and destination handling
- **Workflow integration**: Ensure proper error handling and data flow between modules
- **Test infrastructure fixes**: Resolve environment and assertion issues causing test framework failures

### Non-Functional Requirements
- Maintain backward compatibility with existing module interfaces
- Preserve performance characteristics of file operations
- Ensure cross-platform compatibility (Unix, Windows, macOS)
- Maintain security best practices for file operations
- Support concurrent test execution without interference

### Success Criteria
- All 72 tests in the file operations test suite pass
- No regression in existing functionality
- Test execution time remains under 30 seconds
- Tests are deterministic and non-flaky
- Full coverage of file operation edge cases and error conditions

## API/Interface Design

### Enhanced Stat Module Interface
```rust
pub struct StatResult {
    pub exists: bool,
    pub path: String,
    pub mode: Option<String>,
    pub isreg: bool,
    pub isdir: bool,
    pub islnk: bool,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub owner: Option<String>,
    pub group: Option<String>,
    pub size: u64,
    pub atime: f64,
    pub mtime: f64,
    pub ctime: f64,
    pub checksum: Option<String>,
    pub checksum_algorithm: Option<String>,
    pub lnk_target: Option<String>,
    pub mime_type: Option<String>,
    pub attributes: Option<HashMap<String, serde_json::Value>>,
}

impl StatModule {
    async fn execute(&self, args: &ModuleArgs, context: &ExecutionContext) -> Result<ModuleResult, ModuleExecutionError> {
        let stat_args = StatArgs::from_module_args(args)?;
        let result = self.gather_file_stats(&stat_args).await?;
        
        Ok(ModuleResult {
            changed: false,
            failed: false,
            msg: None,
            stdout: None,
            stderr: None,
            rc: Some(0),
            results: HashMap::new(),
            diff: None,
            warnings: vec![],
            ansible_facts: {
                let mut facts = HashMap::new();
                facts.insert("stat".to_string(), serde_json::to_value(result)?);
                facts
            },
        })
    }
}
```

### Enhanced Copy Module Interface
```rust
pub struct CopyArgs {
    pub src: String,
    pub dest: String,
    pub mode: Option<String>,
    pub owner: Option<String>,
    pub group: Option<String>,
    pub backup: Option<bool>,
    pub force: Option<bool>,
    pub follow: Option<bool>,
    pub preserve: Option<bool>,
    pub directory_mode: Option<String>,
    pub remote_src: Option<bool>,
}

impl CopyModule {
    async fn copy_file(&self, src: &Path, dest: &Path, args: &CopyArgs) -> Result<bool, FileError>;
    async fn copy_directory(&self, src: &Path, dest: &Path, args: &CopyArgs) -> Result<bool, FileError>;
    async fn preserve_attributes(&self, src: &Path, dest: &Path) -> Result<(), FileError>;
    async fn handle_backup(&self, dest: &Path) -> Result<Option<PathBuf>, FileError>;
}
```

### Template Engine Compatibility Layer
```rust
pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();
        
        // Register compatibility helpers for Jinja2-like syntax
        handlebars.register_helper("default", Box::new(default_helper));
        handlebars.register_helper("quote", Box::new(quote_helper));
        
        Self { handlebars }
    }
    
    pub fn render_template(&self, template: &str, variables: &HashMap<String, serde_json::Value>) -> Result<String, TemplateError>;
}

// Compatibility helper for default values
fn default_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _rc: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult;
```

## File and Package Structure

### Enhanced Module Implementation
```
src/modules/files/
├── mod.rs                     # Updated module exports
├── stat.rs                    # Enhanced stat implementation
├── copy.rs                    # Enhanced copy implementation
├── template.rs                # Template engine compatibility
├── utils/
│   ├── checksum.rs           # Enhanced checksum utilities
│   ├── attributes.rs         # File attribute management
│   └── template_helpers.rs   # Handlebars compatibility helpers
└── platform/
    ├── unix.rs               # Unix-specific enhancements
    └── windows.rs            # Windows-specific enhancements
```

### Test Infrastructure Fixes
```
tests/modules/files/
├── helpers/
│   ├── environment.rs        # Fixed test environment
│   ├── assertions.rs         # Enhanced assertions
│   └── compatibility.rs     # Module compatibility helpers
└── fixtures/
    ├── templates/            # Updated template syntax
    └── expected/             # Updated expected outputs
```

## Implementation Details

### 1. Stat Module Enhancement
```rust
// Enhanced stat module implementation
impl StatModule {
    async fn gather_file_stats(&self, args: &StatArgs) -> Result<StatResult, FileError> {
        let path = Path::new(&args.path);
        
        if !path.exists() {
            return Ok(StatResult {
                exists: false,
                path: args.path.clone(),
                ..Default::default()
            });
        }
        
        let metadata = if args.follow.unwrap_or(true) {
            std::fs::metadata(path)?
        } else {
            std::fs::symlink_metadata(path)?
        };
        
        let mut result = StatResult {
            exists: true,
            path: args.path.clone(),
            size: metadata.len(),
            isreg: metadata.is_file(),
            isdir: metadata.is_dir(),
            islnk: metadata.file_type().is_symlink(),
            ..Default::default()
        };
        
        // Add Unix-specific attributes
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            result.mode = Some(format!("{:o}", metadata.mode()));
            result.uid = Some(metadata.uid());
            result.gid = Some(metadata.gid());
            result.atime = metadata.atime() as f64;
            result.mtime = metadata.mtime() as f64;
            result.ctime = metadata.ctime() as f64;
        }
        
        // Calculate checksum if requested
        if args.get_checksum.unwrap_or(false) && result.isreg {
            let algorithm = args.checksum_algorithm.as_deref().unwrap_or("sha1");
            result.checksum = Some(calculate_file_checksum(path, algorithm).await?);
            result.checksum_algorithm = Some(algorithm.to_string());
        }
        
        // Get symlink target if applicable
        if result.islnk {
            if let Ok(target) = std::fs::read_link(path) {
                result.lnk_target = Some(target.to_string_lossy().to_string());
            }
        }
        
        Ok(result)
    }
}
```

### 2. Copy Module Directory Support
```rust
impl CopyModule {
    async fn copy_directory(&self, src: &Path, dest: &Path, args: &CopyArgs) -> Result<bool, FileError> {
        let mut changed = false;
        
        // Create destination directory
        if !dest.exists() {
            fs::create_dir_all(dest).await?;
            changed = true;
        }
        
        // Set directory permissions
        if let Some(mode) = &args.directory_mode {
            set_permissions(dest, mode).await?;
        }
        
        // Recursively copy contents
        let mut entries = fs::read_dir(src).await?;
        while let Some(entry) = entries.next_entry().await? {
            let entry_path = entry.path();
            let dest_path = dest.join(entry.file_name());
            
            if entry_path.is_dir() {
                if self.copy_directory(&entry_path, &dest_path, args).await? {
                    changed = true;
                }
            } else {
                if self.copy_file(&entry_path, &dest_path, args).await? {
                    changed = true;
                }
            }
        }
        
        Ok(changed)
    }
    
    async fn handle_destination(&self, src: &Path, dest: &Path) -> Result<PathBuf, FileError> {
        if dest.is_dir() {
            // Copy into directory with source filename
            if let Some(filename) = src.file_name() {
                Ok(dest.join(filename))
            } else {
                Err(FileError::InvalidPath { 
                    path: src.to_string_lossy().to_string(),
                    reason: "Source has no filename".to_string(),
                })
            }
        } else {
            Ok(dest.to_path_buf())
        }
    }
}
```

### 3. Template Engine Compatibility
```rust
// Handlebars helper for default values
fn default_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _rc: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let value = h.param(0).and_then(|v| v.value().as_str());
    let default = h.param(1).and_then(|v| v.value().as_str()).unwrap_or("");
    
    let result = if let Some(val) = value {
        if val.is_empty() { default } else { val }
    } else {
        default
    };
    
    out.write(result)?;
    Ok(())
}

// Enhanced template processing
impl TemplateModule {
    async fn process_template(&self, args: &TemplateArgs) -> Result<String, TemplateError> {
        let template_content = fs::read_to_string(&args.src).await?;
        
        // Convert Jinja2-like syntax to Handlebars
        let converted_template = self.convert_template_syntax(&template_content)?;
        
        // Prepare context
        let context = json!(args.variables.clone().unwrap_or_default());
        
        // Render template
        self.engine.render_template(&converted_template, &context.as_object().unwrap())
    }
    
    fn convert_template_syntax(&self, template: &str) -> Result<String, TemplateError> {
        // Convert {{ var | default('value') }} to {{default var 'value'}}
        let default_pattern = regex::Regex::new(r"\{\{\s*(\w+)\s*\|\s*default\('([^']*)'\)\s*\}\}")?;
        let converted = default_pattern.replace_all(template, "{{default $1 '$2'}}");
        
        // Convert {{ var | default(value) }} to {{default var value}}
        let default_numeric_pattern = regex::Regex::new(r"\{\{\s*(\w+)\s*\|\s*default\(([^)]*)\)\s*\}\}")?;
        let converted = default_numeric_pattern.replace_all(&converted, "{{default $1 $2}}");
        
        Ok(converted.to_string())
    }
}
```

### 4. Test Infrastructure Fixes
```rust
// Enhanced test environment with proper error handling
impl TestEnvironment {
    pub async fn execute_module_with_context(&self, name: &str, args: ModuleArgs, context: &ExecutionContext) -> Result<ModuleResult> {
        // Implementation with custom context support
        match name {
            "file" => {
                let file_module = FileModule;
                file_module.execute(&args, context).await
            }
            "copy" => {
                let copy_module = CopyModule;
                copy_module.execute(&args, context).await
            }
            "stat" => {
                let stat_module = StatModule;
                stat_module.execute(&args, context).await
            }
            "template" => {
                let template_module = TemplateModule::new();
                template_module.execute(&args, context).await
            }
            _ => anyhow::bail!("Unknown module: {}", name),
        }.map_err(|e| anyhow::anyhow!("Module execution failed: {}", e))
    }
}
```

## Testing Strategy

### Enhanced Test Categories
1. **Stat Module Tests**: Verify ansible_facts format, checksum calculation, all file attributes
2. **Copy Module Tests**: Directory copying, attribute preservation, destination handling
3. **Template Tests**: Handlebars compatibility, variable substitution, conditionals
4. **Integration Tests**: Multi-module workflows with proper error propagation
5. **Error Handling Tests**: Graceful failure modes and recovery

### Test Data Updates
```rust
// Updated template fixtures with Handlebars syntax
impl TestFixtures {
    pub fn load() -> Self {
        // Update templates to use Handlebars-compatible syntax
        fixtures.templates.insert(
            "config".to_string(),
            r#"[app]
name = "{{ app_name }}"
port = {{ port }}
debug = {{ debug }}

[database]
host = "{{default db_host 'localhost'}}"
port = {{default db_port 5432}}
"#.to_string(),
        );
    }
}
```

### Validation Tests
```rust
#[tokio::test]
async fn test_stat_ansible_facts_format() {
    let env = TestEnvironment::new();
    let file_path = env.create_test_file("test.txt", "content");
    
    let args = StatTestBuilder::new()
        .path(file_path.to_string_lossy())
        .build();
    
    let result = env.execute_module("stat", args).await.unwrap();
    
    assert!(!result.failed);
    assert!(!result.ansible_facts.is_empty());
    assert!(result.ansible_facts.contains_key("stat"));
    
    let stat_data = &result.ansible_facts["stat"];
    assert!(stat_data.get("exists").unwrap().as_bool().unwrap());
    assert!(stat_data.get("isreg").unwrap().as_bool().unwrap());
}
```

## Edge Cases & Error Handling

### File System Edge Cases
- Non-existent files and directories
- Permission denied scenarios
- Symlink loops and broken links
- Cross-filesystem copying
- Disk space exhaustion
- Concurrent access conflicts

### Template Engine Edge Cases
```rust
// Handle malformed templates gracefully
impl TemplateModule {
    fn handle_template_error(&self, error: TemplateError) -> ModuleResult {
        ModuleResult {
            changed: false,
            failed: true,
            msg: Some(format!("Template rendering failed: {}", error)),
            stderr: Some(error.to_string()),
            rc: Some(1),
            results: HashMap::new(),
            diff: None,
            warnings: vec![],
            ansible_facts: HashMap::new(),
        }
    }
}
```

### Copy Operation Edge Cases
- Source and destination are the same
- Copying directories into themselves
- Overwriting newer files
- Preserving special file types (devices, FIFOs)
- Cross-platform attribute mapping

## Dependencies

### Enhanced Dependencies
- **handlebars = "6.3"** (already available) - Enhanced with custom helpers
- **regex = "1.10"** (already available) - For template syntax conversion
- **walkdir = "2.4"** (already available) - For directory traversal
- **mime_guess = "2.0"** - For MIME type detection in stat module

### Internal Dependencies
- Enhanced `crate::modules::files::utils::checksum` - Support for multiple algorithms
- Enhanced `crate::modules::files::utils::attributes` - Cross-platform attribute handling
- Enhanced `crate::modules::error` - Specific error types for each failure mode

## Configuration

### Enhanced Module Configuration
```rust
pub struct FileOperationsConfig {
    pub default_checksum_algorithm: String,        // Default: "sha1"
    pub preserve_timestamps: bool,                 // Default: true
    pub follow_symlinks: bool,                     // Default: true
    pub backup_extension: String,                  // Default: ".backup"
    pub max_file_size_for_checksum: u64,          // Default: 100MB
    pub template_syntax_conversion: bool,          // Default: true
}
```

### Test Configuration
```rust
pub struct TestConfig {
    pub timeout_seconds: u64,                     // Default: 30
    pub parallel_execution: bool,                  // Default: true
    pub preserve_temp_files_on_failure: bool,     // Default: false
    pub detailed_error_output: bool,              // Default: true
}
```

## Documentation

### Enhanced Module Documentation
```rust
/// Enhanced stat module that gathers comprehensive file information
/// 
/// Returns file statistics in ansible-compatible format including:
/// - Basic file attributes (size, type, permissions)
/// - Timestamps (access, modification, creation)
/// - Ownership information (user, group)
/// - Optional checksums with configurable algorithms
/// - Symlink target resolution
/// - MIME type detection
/// 
/// # Examples
/// 
/// ```rust
/// let args = StatTestBuilder::new()
///     .path("/path/to/file")
///     .get_checksum(true)
///     .checksum_algorithm("sha256")
///     .build();
/// 
/// let result = stat_module.execute(args, &context).await?;
/// let stat_info = result.ansible_facts["stat"].as_object().unwrap();
/// assert!(stat_info["exists"].as_bool().unwrap());
/// ```
pub struct StatModule;
```

### Test Documentation
```rust
/// Comprehensive test suite for file operations modules
/// 
/// This test suite validates:
/// - Complete module functionality across all platforms
/// - Error handling and edge cases
/// - Performance characteristics
/// - Cross-module integration workflows
/// 
/// Test categories:
/// - Integration tests: Real file system operations
/// - Property tests: Invariant validation
/// - Platform tests: OS-specific behavior
/// - Workflow tests: Multi-module scenarios
mod comprehensive_tests;
```

## Implementation Priority

### Phase 1: Core Fixes (Week 1)
1. Fix stat module ansible_facts format
2. Implement checksum calculation utilities
3. Add basic copy directory support
4. Fix template syntax conversion

### Phase 2: Enhanced Features (Week 2)
5. Complete copy module attribute preservation
6. Add comprehensive file attribute gathering
7. Implement template compatibility helpers
8. Enhance error handling throughout

### Phase 3: Integration & Polish (Week 3)
9. Fix workflow integration tests
10. Add missing test assertions and environment fixes
11. Optimize performance for large file operations
12. Complete cross-platform compatibility

### Phase 4: Validation (Week 4)
13. Achieve 100% test pass rate
14. Performance regression testing
15. Documentation updates
16. Final integration validation

This specification ensures all 25 test failures are systematically addressed while maintaining backward compatibility and enhancing the overall robustness of the file operations module system.