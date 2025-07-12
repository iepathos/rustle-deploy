# Spec 180: API Consolidation and Type System Unification

## Feature Summary

The rustle-deploy project currently suffers from multiple conflicting API definitions, duplicate types, and inconsistent interfaces across the codebase. This specification outlines a comprehensive plan to consolidate APIs, unify the type system, and establish clear module boundaries to resolve compilation issues and improve maintainability.

The consolidation will create a single source of truth for core types, eliminate duplicate definitions, and establish clear ownership of different API layers while maintaining backward compatibility where possible.

## Goals & Requirements

### Primary Goals
- **Eliminate Type Conflicts**: Resolve multiple conflicting definitions of core types like `OptimizationLevel`, `CompiledBinary`, and `TargetSpecification`
- **Establish API Hierarchy**: Create clear boundaries between public APIs, internal APIs, and implementation details
- **Improve Compilation**: Fix all compilation errors in the compilation module and related components
- **Maintain Functionality**: Preserve all existing working functionality during consolidation

### Functional Requirements
- All existing tests must continue to pass after consolidation
- Command-line interface and public APIs must remain stable
- Binary compilation and deployment workflows must work without interruption
- Cross-compilation capabilities must be preserved

### Non-Functional Requirements
- **Performance**: No performance regression in compilation or deployment
- **Maintainability**: Clear separation of concerns and reduced complexity
- **Extensibility**: Easy to add new compilation backends or target platforms
- **Documentation**: All public APIs must be documented

### Success Criteria
- Zero compilation errors across all modules
- All tests pass without warnings
- Clippy produces no warnings
- Documentation builds successfully
- Integration tests demonstrate full functionality

## API/Interface Design

### Core Type Definitions (types/mod.rs)

```rust
// Single source of truth for compilation types
pub mod compilation {
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;
    use std::time::Duration;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum OptimizationLevel {
        Debug,
        Release,
        ReleaseWithDebugInfo,
        MinSize,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CompiledBinary {
        pub compilation_id: String,
        pub target_triple: String,
        pub binary_data: Vec<u8>,
        pub checksum: String,
        pub size: u64,
        pub compilation_time: Duration,
        pub optimization_level: OptimizationLevel,
        pub source_info: BinarySourceInfo,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BinarySourceInfo {
        pub source_type: BinarySourceType,
        pub template_hash: String,
        pub build_metadata: BuildMetadata,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum BinarySourceType {
        Cache { cache_path: PathBuf },
        FreshCompilation { project_path: PathBuf },
        InMemory,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BuildMetadata {
        pub created_at: chrono::DateTime<chrono::Utc>,
        pub toolchain_version: String,
        pub features: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TargetSpecification {
        pub target_triple: String,
        pub optimization_level: OptimizationLevel,
        pub platform_info: PlatformInfo,
        pub compilation_options: CompilationOptions,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PlatformInfo {
        pub architecture: String,
        pub os_family: String,
        pub libc: Option<String>,
        pub features: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CompilationOptions {
        pub strip_debug: bool,
        pub enable_lto: bool,
        pub target_cpu: Option<String>,
        pub custom_features: Vec<String>,
        pub static_linking: bool,
        pub compression: bool,
    }
}
```

### Compilation Backend Interface

```rust
// Abstract compilation backend trait
pub trait CompilationBackend {
    type Error: std::error::Error + Send + Sync + 'static;
    type Config: Default + Clone;

    async fn compile_binary(
        &self,
        template: &GeneratedTemplate,
        target: &TargetSpecification,
        config: &Self::Config,
    ) -> Result<CompiledBinary, Self::Error>;

    fn supports_target(&self, target: &str) -> bool;
    fn get_capabilities(&self) -> BackendCapabilities;
}

#[derive(Debug, Clone)]
pub struct BackendCapabilities {
    pub supported_targets: Vec<String>,
    pub supports_cross_compilation: bool,
    pub supports_static_linking: bool,
    pub supports_lto: bool,
}
```

### Output Management Interface

```rust
pub trait OutputStrategy {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn copy_binary(
        &self,
        binary: &CompiledBinary,
        output_path: &Path,
    ) -> Result<CopyResult, Self::Error>;

    fn can_handle(&self, source_type: &BinarySourceType) -> bool;
}

#[derive(Debug, Clone)]
pub struct CopyResult {
    pub output_path: PathBuf,
    pub bytes_copied: u64,
    pub copy_duration: Duration,
    pub source_verified: bool,
}
```

## File and Package Structure

### Reorganized Module Hierarchy

```
src/
├── types/
│   ├── mod.rs                    # Re-exports all public types
│   ├── compilation.rs            # Core compilation types (canonical)
│   ├── deployment.rs            # Deployment-specific types
│   ├── inventory.rs             # Inventory types
│   └── platform.rs              # Platform detection types
├── compilation/
│   ├── mod.rs                   # Public compilation API
│   ├── backends/
│   │   ├── mod.rs               # Backend registry and selection
│   │   ├── cargo.rs             # Standard Rust compilation
│   │   ├── zigbuild.rs          # Zig-based cross compilation
│   │   └── traits.rs            # Backend trait definitions
│   ├── cache/
│   │   ├── mod.rs               # Cache management
│   │   └── strategies.rs        # Cache storage strategies
│   ├── output/
│   │   ├── mod.rs               # Output management
│   │   └── strategies/          # Output copy strategies
│   ├── target_detection.rs      # Target platform detection
│   └── config.rs                # Compilation configuration
├── deploy/
│   ├── mod.rs                   # High-level deployment API
│   ├── compiler.rs              # Facade over compilation backends
│   ├── manager.rs               # Deployment orchestration
│   └── cache.rs                 # Deployment-specific caching
└── bin/
    └── rustle-deploy.rs         # Updated to use consolidated APIs
```

### Import Strategy

- **Public API**: Only import from `rustle_deploy::types::*` and `rustle_deploy::compilation::*`
- **Internal Modules**: Use explicit paths like `crate::compilation::backends::*`
- **Cross-Module**: Avoid circular dependencies through careful layering

## Implementation Details

### Phase 1: Type Consolidation

1. **Create Canonical Types**
   - Move all type definitions to `types/compilation.rs`
   - Remove duplicate definitions from other modules
   - Update all imports to use canonical types

2. **Migration Strategy**
   - Create type aliases for backward compatibility during transition
   - Add deprecation warnings for old types
   - Update one module at a time to reduce risk

### Phase 2: Backend Unification

1. **Abstract Backend Interface**
   - Define `CompilationBackend` trait
   - Implement for existing Cargo and ZigBuild backends
   - Create backend registry for dynamic selection

2. **Configuration Consolidation**
   - Unify all compilation configuration into `CompilationOptions`
   - Remove duplicate configuration structs
   - Implement conversion functions for backward compatibility

### Phase 3: API Cleanup

1. **Public API Definition**
   - Clearly mark public vs internal APIs
   - Add comprehensive documentation
   - Create facade types for complex internal types

2. **Error Handling Standardization**
   - Create unified error types for each subsystem
   - Implement proper error conversion chains
   - Add context to all error messages

### Phase 4: Output System Refactor

1. **Strategy Pattern Implementation**
   - Implement `OutputStrategy` trait for all copy methods
   - Create registry for strategy selection
   - Unify `CopyResult` type across all strategies

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimization_level_serialization() {
        let level = OptimizationLevel::MinSize;
        let serialized = serde_json::to_string(&level).unwrap();
        let deserialized: OptimizationLevel = serde_json::from_str(&serialized).unwrap();
        assert_eq!(level, deserialized);
    }

    #[test]
    fn test_target_specification_creation() {
        let target = TargetSpecification {
            target_triple: "x86_64-unknown-linux-gnu".to_string(),
            optimization_level: OptimizationLevel::Release,
            platform_info: PlatformInfo {
                architecture: "x86_64".to_string(),
                os_family: "unix".to_string(),
                libc: Some("gnu".to_string()),
                features: vec![],
            },
            compilation_options: CompilationOptions::default(),
        };
        assert_eq!(target.target_triple, "x86_64-unknown-linux-gnu");
    }
}
```

### Integration Tests

- **Backend Compatibility**: Test all backends produce compatible binaries
- **Type Conversion**: Test all legacy type conversions work correctly
- **API Stability**: Test public API contracts remain stable
- **Cross-Module**: Test modules interact correctly through new APIs

### Migration Tests

```rust
#[test]
fn test_legacy_type_compatibility() {
    // Test that old code using legacy types still works
    let legacy_binary = legacy::CompiledBinary { /* ... */ };
    let new_binary: CompiledBinary = legacy_binary.into();
    assert!(new_binary.compilation_id.len() > 0);
}
```

## Edge Cases & Error Handling

### Type Migration Issues

1. **Field Missing**: Handle cases where legacy types are missing fields
2. **Type Conversion**: Graceful conversion between different enum variants
3. **Serialization**: Maintain compatibility with existing serialized data

### Backend Compatibility

1. **Missing Backend**: Fallback to available backend when preferred is unavailable
2. **Target Unsupported**: Clear error messages for unsupported target platforms
3. **Configuration Conflict**: Resolve conflicting compilation options

### Cache Corruption

1. **Version Mismatch**: Handle cache entries from different type versions
2. **Data Corruption**: Validate cache entries and rebuild when corrupted
3. **Disk Space**: Handle insufficient disk space during cache operations

## Dependencies

### New Dependencies

```toml
[dependencies]
# Existing dependencies remain
thiserror = "1.0"     # For unified error handling
serde = { version = "1.0", features = ["derive"] }
chrono = { version = "0.4", features = ["serde"] }
```

### Internal Dependencies

- **types** module becomes foundation for all other modules
- **compilation** module depends only on types and external crates
- **deploy** module orchestrates compilation and output management
- **binary** depends on compilation for binary analysis

## Configuration

### Compilation Configuration

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationConfig {
    pub default_optimization: OptimizationLevel,
    pub preferred_backend: Option<String>,
    pub cache_settings: CacheConfig,
    pub output_settings: OutputConfig,
    pub target_settings: TargetConfig,
}

impl Default for CompilationConfig {
    fn default() -> Self {
        Self {
            default_optimization: OptimizationLevel::Release,
            preferred_backend: None,
            cache_settings: CacheConfig::default(),
            output_settings: OutputConfig::default(),
            target_settings: TargetConfig::default(),
        }
    }
}
```

### Environment Variables

- `RUSTLE_COMPILATION_BACKEND`: Override default compilation backend
- `RUSTLE_CACHE_DIR`: Override default cache directory
- `RUSTLE_TARGET_OVERRIDE`: Force specific target platform
- `RUSTLE_OPTIMIZATION_LEVEL`: Override optimization level

## Documentation

### Public API Documentation

```rust
/// Core compilation types and interfaces for rustle-deploy.
/// 
/// This module provides the canonical types for compilation operations,
/// including target specifications, optimization levels, and binary metadata.
/// All other modules should import types from this module to ensure consistency.
/// 
/// # Examples
/// 
/// ```rust
/// use rustle_deploy::types::compilation::{OptimizationLevel, TargetSpecification};
/// 
/// let target = TargetSpecification {
///     target_triple: "x86_64-unknown-linux-gnu".to_string(),
///     optimization_level: OptimizationLevel::Release,
///     // ...
/// };
/// ```
pub mod compilation {
    // ...
}
```

### Migration Guide

Create comprehensive migration guide in `docs/api-migration.md`:
- Mapping from old types to new types
- Code examples for common migration patterns
- Deprecation timeline
- Breaking changes and workarounds

### README Updates

- Update architecture diagrams to show new module structure
- Add API stability guarantees
- Document supported compilation backends
- Update example code to use new APIs

## Backward Compatibility Strategy

### Transition Period

1. **Phase 1** (Current): Both old and new APIs available
2. **Phase 2** (1 month): Deprecation warnings for old APIs
3. **Phase 3** (2 months): Old APIs marked as deprecated
4. **Phase 4** (3 months): Old APIs removed

### Compatibility Layer

```rust
// Provide compatibility aliases during transition
#[deprecated(since = "0.2.0", note = "Use rustle_deploy::types::compilation::OptimizationLevel")]
pub use crate::types::compilation::OptimizationLevel as CompilationOptimizationLevel;

#[deprecated(since = "0.2.0", note = "Use rustle_deploy::types::compilation::CompiledBinary")]
pub use crate::types::compilation::CompiledBinary as LegacyCompiledBinary;
```

### Feature Flags

```toml
[features]
default = ["new-api"]
new-api = []
legacy-api = []
```

## Performance Considerations

### Type System Performance

- Use `#[repr(C)]` for types that cross FFI boundaries
- Implement `Clone` efficiently with `Arc` for large data structures
- Use `Cow<str>` for strings that might be borrowed or owned

### Compilation Pipeline Performance

- Maintain zero-copy semantics where possible
- Use async/await for I/O operations
- Implement proper caching to avoid redundant work

### Memory Usage

- Use streaming for large binary data
- Implement proper cleanup for temporary files
- Monitor memory usage in compilation backends

## Risk Assessment

### High Risk
- **Breaking Changes**: Public API changes could break existing code
- **Data Migration**: Existing cache and serialized data might become incompatible

### Medium Risk
- **Performance Regression**: New abstraction layers might impact performance
- **Backend Issues**: Unifying backends might expose existing bugs

### Low Risk
- **Documentation**: Some documentation might become outdated
- **Testing**: Some tests might need updates

## Implementation Timeline

### Week 1: Foundation
- Create canonical types in `types/compilation.rs`
- Set up new module structure
- Add compatibility layer

### Week 2: Backend Unification
- Implement `CompilationBackend` trait
- Migrate existing backends to new interface
- Update configuration system

### Week 3: API Cleanup
- Update all internal modules to use new types
- Fix compilation errors
- Update tests

### Week 4: Integration & Testing
- Run comprehensive test suite
- Performance testing
- Documentation updates
- Prepare for rollout

## Success Metrics

- **Zero compilation errors**: All modules compile without errors
- **Test coverage**: Maintain >90% test coverage
- **Performance**: No more than 5% performance regression
- **Documentation**: All public APIs documented with examples
- **Adoption**: Internal modules successfully migrated to new APIs