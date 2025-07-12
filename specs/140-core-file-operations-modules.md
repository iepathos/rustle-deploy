# Spec 140: Core File Operations Modules

## Feature Summary

Implement essential file operation modules (file, copy, stat, template) within the rustle-deploy binary execution system that provide the foundation for configuration management and deployment automation. These modules are critical for any deployment tool as they handle the fundamental operations of managing files, copying content, checking file states, and processing configuration templates.

**Architecture Note**: These modules are self-contained within rustle-deploy, following Unix philosophy of tool independence. They do not depend on external rustle CLI tools.

## Goals & Requirements

### Functional Requirements
- **file module**: Set file/directory attributes (permissions, ownership, state, links)
- **copy module**: Copy files from source to destination with validation and backup
- **stat module**: Gather file/directory metadata and existence information
- **template module**: Process templates with variable substitution using Handlebars

### Non-Functional Requirements
- Cross-platform compatibility (Linux, macOS, Windows, *BSD)
- Atomic operations where possible (temp files + rename)
- Comprehensive error handling with detailed messages
- Support for check mode (dry-run) operations
- Performance optimization for large files
- Security-conscious file permission handling

### Success Criteria
- All four modules pass comprehensive test suites
- Cross-platform functionality verified on major platforms
- Integration with existing module architecture
- Documentation with examples for each module
- Performance benchmarks for file operations

## API/Interface Design

### File Module Interface
```rust
#[async_trait]
impl ExecutionModule for FileModule {
    async fn execute(&self, args: &ModuleArgs, context: &ExecutionContext) -> Result<ModuleResult, ModuleExecutionError>;
}

// Arguments
pub struct FileArgs {
    pub path: String,                    // Required: target file/directory path
    pub state: FileState,                // present, absent, directory, link, hard, touch
    pub mode: Option<String>,            // File permissions (0644, u+rwx, etc.)
    pub owner: Option<String>,           // File owner (username or UID)
    pub group: Option<String>,           // File group (groupname or GID)
    pub src: Option<String>,             // Source for link operations
    pub recurse: Option<bool>,           // Recursive operations for directories
    pub follow: Option<bool>,            // Follow symlinks
    pub force: Option<bool>,             // Force operations
    pub backup: Option<bool>,            // Create backup before changes
}

pub enum FileState {
    Present,    // Ensure file exists
    Absent,     // Ensure file doesn't exist
    Directory,  // Ensure directory exists
    Link,       // Create symbolic link
    Hard,       // Create hard link
    Touch,      // Touch file (update timestamp)
}
```

### Copy Module Interface
```rust
pub struct CopyArgs {
    pub src: String,                     // Required: source file path
    pub dest: String,                    // Required: destination path
    pub backup: Option<bool>,            // Create backup of destination
    pub force: Option<bool>,             // Overwrite existing files
    pub mode: Option<String>,            // Set permissions on copied file
    pub owner: Option<String>,           // Set owner on copied file
    pub group: Option<String>,           // Set group on copied file
    pub directory_mode: Option<String>,  // Permissions for created directories
    pub validate: Option<String>,        // Command to validate copied file
    pub checksum: Option<String>,        // Expected checksum of source
}
```

### Stat Module Interface
```rust
pub struct StatArgs {
    pub path: String,                    // Required: path to examine
    pub follow: Option<bool>,            // Follow symlinks
    pub get_checksum: Option<bool>,      // Calculate file checksum
    pub checksum_algorithm: Option<String>, // sha1, sha256, md5
}

pub struct StatResult {
    pub exists: bool,
    pub path: String,
    pub mode: String,
    pub isdir: bool,
    pub isreg: bool,
    pub islnk: bool,
    pub size: u64,
    pub uid: u32,
    pub gid: u32,
    pub owner: String,
    pub group: String,
    pub mtime: f64,
    pub atime: f64,
    pub ctime: f64,
    pub checksum: Option<String>,
    pub link_target: Option<String>,
}
```

### Template Module Interface
```rust
pub struct TemplateArgs {
    pub src: String,                     // Required: template file path
    pub dest: String,                    // Required: destination file path
    pub backup: Option<bool>,            // Backup destination before writing
    pub mode: Option<String>,            // File permissions
    pub owner: Option<String>,           // File owner
    pub group: Option<String>,           // File group
    pub validate: Option<String>,        // Validation command
    pub variables: Option<serde_json::Value>, // Template variables
}
```

## File and Package Structure

### Module Organization
```
src/modules/files/
├── mod.rs                    # Module declarations and exports
├── file.rs                   # File attributes module
├── copy.rs                   # File copying module
├── stat.rs                   # File information module
├── template.rs               # Template processing module
├── utils/
│   ├── mod.rs               # Utilities module
│   ├── permissions.rs       # Cross-platform permission handling
│   ├── ownership.rs         # User/group resolution
│   ├── backup.rs            # Backup file operations
│   ├── atomic.rs            # Atomic file operations
│   └── checksum.rs          # File checksum calculations
└── platform/
    ├── mod.rs               # Platform-specific implementations
    ├── unix.rs              # Unix-like platform operations
    └── windows.rs           # Windows platform operations
```

### Integration Points
- Update `src/modules/mod.rs` to include files module
- Register modules in module registry
- Add template modules to `src/templates/modules/`

## Implementation Details

### 1. Cross-Platform Permission Handling
```rust
#[cfg(unix)]
mod unix_permissions {
    use nix::sys::stat::{Mode, fchmod};
    use std::os::unix::fs::PermissionsExt;
    
    pub fn set_permissions(path: &Path, mode: &str) -> Result<(), FileError> {
        let mode = parse_mode(mode)?;
        let file = std::fs::File::open(path)?;
        fchmod(file.as_raw_fd(), Mode::from_bits_truncate(mode))?;
        Ok(())
    }
}

#[cfg(windows)]
mod windows_permissions {
    use winapi::um::fileapi::SetFileAttributesW;
    
    pub fn set_permissions(path: &Path, mode: &str) -> Result<(), FileError> {
        // Windows-specific permission handling
        // Map Unix-style permissions to Windows ACLs where possible
    }
}
```

### 2. Atomic File Operations
```rust
pub struct AtomicWriter {
    temp_path: PathBuf,
    final_path: PathBuf,
    temp_file: File,
}

impl AtomicWriter {
    pub fn new(target_path: impl AsRef<Path>) -> Result<Self, FileError> {
        let final_path = target_path.as_ref().to_path_buf();
        let temp_path = create_temp_file_path(&final_path)?;
        let temp_file = File::create(&temp_path)?;
        
        Ok(AtomicWriter {
            temp_path,
            final_path,
            temp_file,
        })
    }
    
    pub async fn commit(self) -> Result<(), FileError> {
        drop(self.temp_file);
        tokio::fs::rename(&self.temp_path, &self.final_path).await?;
        Ok(())
    }
}
```

### 3. Template Processing Integration
```rust
use handlebars::Handlebars;

pub struct TemplateProcessor {
    handlebars: Handlebars<'static>,
}

impl TemplateProcessor {
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);
        
        Self { handlebars }
    }
    
    pub fn render_template(
        &self,
        template_content: &str,
        variables: &serde_json::Value,
    ) -> Result<String, TemplateError> {
        self.handlebars.render_template(template_content, variables)
            .map_err(TemplateError::from)
    }
}
```

### 4. Checksum Validation
```rust
use sha2::{Sha256, Digest};

pub async fn calculate_file_checksum(
    path: &Path,
    algorithm: ChecksumAlgorithm,
) -> Result<String, FileError> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = match algorithm {
        ChecksumAlgorithm::Sha256 => Sha256::new(),
        ChecksumAlgorithm::Sha1 => sha1::Sha1::new(),
        ChecksumAlgorithm::Md5 => md5::Md5::new(),
    };
    
    let mut buffer = vec![0; 8192];
    loop {
        let bytes_read = file.read(&mut buffer).await?;
        if bytes_read == 0 { break; }
        hasher.update(&buffer[..bytes_read]);
    }
    
    Ok(format!("{:x}", hasher.finalize()))
}
```

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_file_create() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file.txt");
        
        let args = ModuleArgs {
            args: json!({
                "path": file_path.to_string_lossy(),
                "state": "touch"
            }).as_object().unwrap().clone()
        };
        
        let module = FileModule;
        let context = ExecutionContext::new(false);
        let result = module.execute(&args, &context).await.unwrap();
        
        assert!(result.changed);
        assert!(file_path.exists());
    }
    
    #[tokio::test]
    async fn test_copy_with_backup() {
        // Test file copying with backup functionality
    }
    
    #[tokio::test]
    async fn test_template_rendering() {
        // Test template processing with variables
    }
    
    #[tokio::test]
    async fn test_stat_file_info() {
        // Test file stat information gathering
    }
}
```

### Integration Tests
```rust
// tests/modules/files_integration_tests.rs
#[tokio::test]
async fn test_file_operations_workflow() {
    // Test complete workflow: create dir, copy file, set permissions, template processing
}

#[tokio::test]
async fn test_cross_platform_permissions() {
    // Test permission handling across different platforms
}
```

### Property-Based Tests
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_checksum_consistency(content in ".*") {
        // Verify checksum calculations are consistent
    }
    
    #[test]
    fn test_atomic_operations(path in valid_path_strategy()) {
        // Test atomic operations never leave partial states
    }
}
```

## Edge Cases & Error Handling

### File System Limitations
- Handle path length limits on different platforms
- Deal with special characters in file names
- Handle case-sensitive vs case-insensitive file systems
- Manage file system permission models (Unix vs Windows)

### Concurrency and Locking
- Handle file locking on Windows
- Manage concurrent access to same files
- Atomic operations for safety

### Error Recovery
```rust
#[derive(thiserror::Error, Debug)]
pub enum FileError {
    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },
    
    #[error("File not found: {path}")]
    NotFound { path: String },
    
    #[error("Invalid permissions format: {mode}")]
    InvalidPermissions { mode: String },
    
    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },
    
    #[error("Template rendering failed: {source}")]
    TemplateError { source: handlebars::RenderError },
    
    #[error("IO error: {source}")]
    Io { source: std::io::Error },
}
```

## Dependencies

### External Crates
- `handlebars = "6.3"` (already available) - Template processing
- `sha2 = "0.10"` (already available) - Checksum calculations
- `nix = "0.30"` (already available for Unix) - Unix system calls
- `winapi = "0.3"` (already available for Windows) - Windows APIs
- `tempfile = "3"` (already available) - Temporary file handling

### Internal Dependencies
- `crate::modules::interface` - Module interface traits
- `crate::modules::error` - Error handling types
- `crate::types::platform` - Platform detection

## Configuration

### Module Configuration
```rust
pub struct FileModuleConfig {
    pub default_backup_suffix: String,    // Default: ".backup"
    pub max_file_size: Option<u64>,        // Maximum file size for operations
    pub checksum_algorithm: ChecksumAlgorithm, // Default: Sha256
    pub atomic_operations: bool,           // Default: true
    pub follow_symlinks: bool,             // Default: false
}
```

### Environment Variables
- `RUSTLE_FILES_BACKUP_SUFFIX` - Default backup file suffix
- `RUSTLE_FILES_MAX_SIZE` - Maximum file size for operations
- `RUSTLE_FILES_CHECKSUM_ALGO` - Default checksum algorithm

## Documentation

### Module Documentation
Each module requires comprehensive documentation:
- Purpose and use cases
- Parameter reference with examples
- Return value specifications
- Platform-specific behavior notes
- Security considerations
- Performance characteristics

### Example Usage
```yaml
# File module examples
- name: Create directory
  file:
    path: /etc/myapp
    state: directory
    mode: '0755'
    owner: root
    group: root

- name: Create symbolic link
  file:
    src: /usr/local/bin/myapp
    dest: /usr/bin/myapp
    state: link

# Copy module example
- name: Copy configuration file
  copy:
    src: myapp.conf
    dest: /etc/myapp/myapp.conf
    backup: yes
    mode: '0644'
    validate: 'myapp --test-config %s'

# Template module example
- name: Generate configuration from template
  template:
    src: nginx.conf.j2
    dest: /etc/nginx/nginx.conf
    backup: yes
    variables:
      server_name: "{{ inventory_hostname }}"
      worker_processes: "{{ ansible_processor_vcpus }}"

# Stat module example
- name: Check if file exists
  stat:
    path: /etc/myapp/myapp.conf
  register: config_file
```

## Implementation Priority

### Phase 1: Foundation (Week 1)
1. File utilities (permissions, ownership, atomic operations)
2. Basic file module (create, delete, touch operations)
3. Cross-platform abstraction layer

### Phase 2: Core Operations (Week 2)
4. Copy module with basic functionality
5. Stat module for file information
6. Enhanced file module (permissions, ownership)

### Phase 3: Advanced Features (Week 3)
7. Template module with Handlebars integration
8. Backup functionality across all modules
9. Checksum validation and verification

### Phase 4: Polish & Testing (Week 4)
10. Comprehensive test coverage
11. Performance optimization
12. Documentation and examples
13. Cross-platform validation

This specification provides the foundation for essential file operations that will enable comprehensive configuration management and deployment automation capabilities in the rustle-deploy system.