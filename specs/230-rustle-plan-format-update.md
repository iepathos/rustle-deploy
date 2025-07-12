# Spec 230: Rustle-Plan Format Compatibility Update

## Feature Summary

Update rustle-deploy to be compatible with the latest rustle-plan output format. The new format introduces significant changes to the `BinaryDeploymentPlan` structure, including new fields like `binary_name`, `embedded_data`, and updated `compilation_requirements`. This spec addresses the compatibility gap identified between the current rustle-deploy parsing logic and the new rustle-plan output format.

The updated format provides richer metadata for binary deployments, including embedded execution plans, static file references, and enhanced compilation requirements that support cross-platform deployment scenarios.

## Goals & Requirements

### Functional Requirements
- Parse new rustle-plan output format without breaking existing functionality
- Support new `embedded_data` structure with execution plans and static files
- Handle updated `compilation_requirements` format with architecture-specific fields
- Maintain backward compatibility where possible
- Support new binary deployment metadata fields

### Non-functional Requirements
- Zero performance degradation in parsing existing valid plans
- Maintain memory efficiency when handling large embedded execution plans
- Preserve type safety with strongly-typed Rust structs
- Error messages should clearly indicate format incompatibilities

### Success Criteria
- All existing tests pass with updated format
- New test fixtures using latest format parse successfully
- Binary deployment plans correctly extract embedded data
- Compilation requirements properly handle new architecture fields

## API/Interface Design

### Updated BinaryDeploymentPlan Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryDeploymentPlan {
    pub deployment_id: String,
    pub target_hosts: Vec<String>,
    pub binary_name: String,                    // NEW
    pub tasks: Vec<String>,                     // RENAMED from task_ids
    pub modules: Vec<String>,                   // NEW
    pub embedded_data: EmbeddedData,            // NEW
    pub execution_mode: ExecutionMode,          // NEW
    pub estimated_size: u64,                    // NEW
    pub compilation_requirements: CompilationRequirements, // UPDATED
    
    // Legacy fields (deprecated but maintained for compatibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_ids: Option<Vec<String>>,          // For backward compatibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_architecture: Option<String>,   // For backward compatibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_savings: Option<Duration>,   // For backward compatibility
    
    // Existing template generation fields (unchanged)
    pub controller_endpoint: Option<String>,
    #[serde(with = "serde_duration_opt")]
    pub execution_timeout: Option<Duration>,
    #[serde(with = "serde_duration_opt")]
    pub report_interval: Option<Duration>,
    pub cleanup_on_completion: Option<bool>,
    pub log_level: Option<String>,
    pub max_retries: Option<u32>,
    pub static_files: Vec<StaticFileRef>,
    pub secrets: Vec<SecretRef>,
    pub verbose: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedData {
    pub execution_plan: String,                 // JSON string of execution plan
    pub static_files: Vec<EmbeddedStaticFile>,
    pub variables: HashMap<String, serde_json::Value>,
    pub facts_required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedStaticFile {
    pub src_path: String,
    pub dest_path: String,
    pub checksum: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionMode {
    Controller,
    Standalone,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationRequirements {
    // New format fields
    pub target_arch: String,
    pub target_os: String,
    pub rust_version: String,
    pub cross_compilation: bool,
    pub static_linking: bool,
    
    // Legacy fields (deprecated but maintained for compatibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modules: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub static_files: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_triple: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimization_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<String>>,
}
```

### Migration Helper Functions

```rust
impl BinaryDeploymentPlan {
    /// Convert legacy task_ids to new tasks format
    pub fn migrate_task_ids(&mut self) {
        if let Some(task_ids) = &self.task_ids {
            if self.tasks.is_empty() {
                self.tasks = task_ids.clone();
            }
        }
    }
    
    /// Extract target architecture from legacy or new format
    pub fn get_target_architecture(&self) -> String {
        if !self.compilation_requirements.target_arch.is_empty() {
            format!("{}-{}", 
                self.compilation_requirements.target_arch,
                self.compilation_requirements.target_os
            )
        } else if let Some(arch) = &self.target_architecture {
            arch.clone()
        } else {
            "unknown".to_string()
        }
    }
    
    /// Parse embedded execution plan as JSON
    pub fn parse_execution_plan(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::from_str(&self.embedded_data.execution_plan)
    }
}

impl CompilationRequirements {
    /// Create from legacy format
    pub fn from_legacy(
        modules: Vec<String>,
        target_triple: String,
        optimization_level: String,
        features: Vec<String>,
    ) -> Self {
        let (arch, os) = Self::parse_target_triple(&target_triple);
        
        Self {
            target_arch: arch,
            target_os: os,
            rust_version: "1.70.0".to_string(), // Default
            cross_compilation: false,
            static_linking: true,
            modules: Some(modules),
            static_files: None,
            target_triple: Some(target_triple),
            optimization_level: Some(optimization_level),
            features: Some(features),
        }
    }
    
    fn parse_target_triple(triple: &str) -> (String, String) {
        let parts: Vec<&str> = triple.split('-').collect();
        if parts.len() >= 3 {
            (parts[0].to_string(), parts[2].to_string())
        } else {
            ("x86_64".to_string(), "linux".to_string())
        }
    }
}
```

## File and Package Structure

### Primary Changes
- `src/execution/rustle_plan.rs` - Update struct definitions
- `src/execution/compatibility.rs` - Add format migration logic
- `tests/fixtures/execution_plans/` - Update test fixtures
- `tests/execution/rustle_plan_tests.rs` - Add compatibility tests

### New Files
- `src/execution/format_migration.rs` - Format migration utilities
- `tests/fixtures/execution_plans/new_format_examples/` - New format test cases

## Implementation Details

### Phase 1: Update Core Structures

1. **Update BinaryDeploymentPlan in rustle_plan.rs**
   ```rust
   // Add new fields with proper serde annotations
   // Mark legacy fields as optional for backward compatibility
   // Implement Default trait for new fields
   ```

2. **Add EmbeddedData and related structs**
   ```rust
   // Create new structs for embedded data
   // Implement proper JSON handling for execution_plan field
   // Add validation for embedded static files
   ```

3. **Update CompilationRequirements**
   ```rust
   // Add new architecture-specific fields
   // Maintain legacy fields as optional
   // Implement conversion between formats
   ```

### Phase 2: Implement Migration Logic

1. **Create format_migration.rs module**
   ```rust
   pub struct FormatMigrator {
       pub fn migrate_binary_deployment_plan(
           &self, 
           plan: &mut BinaryDeploymentPlan
       ) -> Result<(), MigrationError> {
           // Handle task_ids -> tasks migration
           // Convert legacy compilation requirements
           // Populate default values for new fields
       }
   }
   ```

2. **Update parsing logic in parser.rs**
   ```rust
   // Add migration step after initial parsing
   // Handle both old and new formats gracefully
   // Provide clear error messages for unsupported formats
   ```

### Phase 3: Enhanced Error Handling

1. **Add specific error types**
   ```rust
   #[derive(Debug, thiserror::Error)]
   pub enum FormatCompatibilityError {
       #[error("Unsupported format version: {version}")]
       UnsupportedVersion { version: String },
       
       #[error("Missing required field in new format: {field}")]
       MissingRequiredField { field: String },
       
       #[error("Invalid embedded execution plan: {reason}")]
       InvalidEmbeddedPlan { reason: String },
   }
   ```

2. **Update validation logic**
   ```rust
   // Validate new format requirements
   // Check embedded data consistency
   // Verify compilation requirements completeness
   ```

### Phase 4: Testing Strategy Updates

1. **Create new test fixtures**
   - Copy updated file_operations_plan.json as reference
   - Create minimal examples for each new field
   - Add edge cases for format migration

2. **Update existing tests**
   - Ensure backward compatibility tests pass
   - Add migration-specific test cases
   - Test error handling for invalid formats

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_new_format_parsing() {
        let json = include_str!("../fixtures/execution_plans/new_format_example.json");
        let plan: RustlePlanOutput = serde_json::from_str(json).unwrap();
        
        assert!(!plan.binary_deployments.is_empty());
        let deployment = &plan.binary_deployments[0];
        
        assert_eq!(deployment.binary_name, "rustle-runner-group_0");
        assert!(!deployment.embedded_data.execution_plan.is_empty());
        assert_eq!(deployment.compilation_requirements.target_arch, "x86_64");
    }
    
    #[test]
    fn test_legacy_format_migration() {
        let mut deployment = create_legacy_deployment_plan();
        let migrator = FormatMigrator::new();
        
        migrator.migrate_binary_deployment_plan(&mut deployment).unwrap();
        
        assert!(!deployment.tasks.is_empty());
        assert!(!deployment.compilation_requirements.target_arch.is_empty());
    }
    
    #[test]
    fn test_embedded_execution_plan_parsing() {
        let deployment = create_new_format_deployment();
        let execution_plan = deployment.parse_execution_plan().unwrap();
        
        assert!(execution_plan.is_object());
        assert!(execution_plan["tasks"].is_array());
    }
}
```

### Integration Tests

```rust
#[test]
fn test_end_to_end_new_format_processing() {
    // Load new format plan
    // Process through full pipeline
    // Verify binary generation works
    // Check embedded data extraction
}

#[test]
fn test_backward_compatibility_full_pipeline() {
    // Load legacy format plan
    // Process with migration enabled
    // Verify equivalent output to new format
}
```

### Property Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_format_migration_idempotent(
        deployment in any::<BinaryDeploymentPlan>()
    ) {
        let mut deployment1 = deployment.clone();
        let mut deployment2 = deployment;
        
        let migrator = FormatMigrator::new();
        migrator.migrate_binary_deployment_plan(&mut deployment1).unwrap();
        migrator.migrate_binary_deployment_plan(&mut deployment2).unwrap();
        
        // Second migration should be no-op
        assert_eq!(deployment1, deployment2);
    }
}
```

## Edge Cases & Error Handling

### Format Detection
- **Missing new fields**: Treat as legacy format and migrate
- **Partial new format**: Validate required fields and error clearly
- **Invalid embedded data**: Provide specific error messages
- **Architecture mismatch**: Handle cross-compilation scenarios

### Migration Edge Cases
- **Empty task lists**: Handle gracefully with defaults
- **Invalid JSON in execution_plan**: Catch and report parsing errors
- **Conflicting legacy/new fields**: Prioritize new format values
- **Missing compilation requirements**: Provide sensible defaults

### Error Recovery
```rust
impl FormatMigrator {
    pub fn migrate_with_fallback(
        &self,
        plan: &mut BinaryDeploymentPlan
    ) -> Result<Vec<MigrationWarning>, MigrationError> {
        let mut warnings = Vec::new();
        
        // Try to migrate each component independently
        if let Err(e) = self.migrate_tasks(plan) {
            warnings.push(MigrationWarning::TaskMigrationFailed(e));
            // Use fallback logic
        }
        
        // Continue with other migrations...
        Ok(warnings)
    }
}
```

## Dependencies

### Internal Dependencies
- `serde` and `serde_json` for serialization (existing)
- `thiserror` for error handling (existing)
- `chrono` for timestamps (existing)

### New Dependencies
- No new external dependencies required
- Leverages existing rustle-deploy infrastructure

### Version Requirements
- Maintain compatibility with Rust 1.70.0+
- Compatible with existing serde ecosystem
- No breaking changes to public APIs

## Configuration

### Feature Flags
```toml
[features]
default = ["new-format-support"]
new-format-support = []
legacy-format-only = []
strict-validation = []
```

### Migration Settings
```rust
#[derive(Debug, Clone)]
pub struct MigrationConfig {
    pub strict_mode: bool,           // Fail on any migration issues
    pub preserve_legacy_fields: bool, // Keep old fields for debugging
    pub validate_embedded_data: bool, // Deep validation of embedded content
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            strict_mode: false,
            preserve_legacy_fields: true,
            validate_embedded_data: true,
        }
    }
}
```

## Documentation

### GoDoc Requirements
```rust
/// Updates rustle-deploy to support the latest rustle-plan output format.
/// 
/// This module provides compatibility between legacy and new format versions,
/// with automatic migration capabilities and comprehensive error handling.
/// 
/// # Examples
/// 
/// ```rust
/// use rustle_deploy::execution::format_migration::FormatMigrator;
/// 
/// let migrator = FormatMigrator::new();
/// let mut plan = load_legacy_plan()?;
/// migrator.migrate_binary_deployment_plan(&mut plan)?;
/// 
/// // Plan is now compatible with new format
/// assert!(!plan.embedded_data.execution_plan.is_empty());
/// ```
/// 
/// # Migration Process
/// 
/// 1. **Field Migration**: Maps legacy field names to new structure
/// 2. **Data Enhancement**: Adds default values for new required fields  
/// 3. **Validation**: Ensures migrated data meets new format requirements
/// 4. **Error Recovery**: Provides fallback options for migration failures
```

### README Updates
- Document new format support in feature list
- Add migration guide for users upgrading from legacy format
- Include troubleshooting section for format compatibility issues

### API Documentation
- Document all new struct fields and their purposes
- Provide examples of embedded data usage
- Explain compilation requirements changes

## Implementation Timeline

### Phase 1 (Week 1): Core Structure Updates
- Update BinaryDeploymentPlan and related structs
- Implement basic serialization/deserialization
- Add new test fixtures

### Phase 2 (Week 1): Migration Logic
- Implement FormatMigrator
- Add backward compatibility handling
- Create migration tests

### Phase 3 (Week 2): Integration & Testing
- Update parsing pipeline to use migration
- Comprehensive test suite
- Performance validation

### Phase 4 (Week 2): Documentation & Polish
- Complete documentation
- Error message improvements
- Final integration testing

## Risk Mitigation

### Breaking Changes
- Use feature flags to allow gradual migration
- Maintain legacy support until next major version
- Provide clear migration path documentation

### Performance Impact
- Lazy migration only when needed
- Cache migrated plans to avoid repeated work
- Profile memory usage with large embedded plans

### Compatibility Issues
- Extensive test matrix covering format combinations
- Clear error messages for unsupported scenarios
- Fallback mechanisms for partial migrations