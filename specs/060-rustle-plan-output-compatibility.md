# Spec 060: Rustle Plan Output Compatibility

## Feature Summary

Implement full compatibility with the actual rustle-plan output format to enable rustle-deploy to process real execution plans. This bridges the critical gap between the theoretical execution plan format defined in spec 020 and the actual JSON structure that rustle-plan generates.

**Problem it solves**: The current ExecutionPlan data structures in spec 020 don't match the actual rustle-plan output format shown in `example_rustle_plan_output.json`, making rustle-deploy unable to process real execution plans from the pipeline.

**High-level approach**: Update the execution plan data structures to match the actual rustle-plan format, implement backward compatibility, and add robust parsing for the real-world JSON structure with binary deployment analysis.

## Goals & Requirements

### Functional Requirements
- Parse actual rustle-plan JSON output format accurately
- Support the play-based structure with batches and tasks
- Handle binary deployment detection from task analysis
- Extract metadata, hosts, and execution parameters
- Support conditional execution and task dependencies
- Process handler definitions and notifications
- Maintain backward compatibility with existing code
- Generate binary deployment opportunities from task analysis
- Support parallelism and network efficiency scoring
- Handle risk assessment and duration estimation

### Non-functional Requirements
- **Performance**: Parse 1000+ task execution plans in <200ms
- **Reliability**: 99.9%+ parsing success rate for valid rustle-plan output
- **Memory**: Efficient parsing for execution plans up to 500MB
- **Compatibility**: Support all rustle-plan v1.0+ output features
- **Validation**: Comprehensive error reporting for malformed plans

### Success Criteria
- Successfully parse all valid rustle-plan output JSON
- Extract binary deployment opportunities automatically
- Enable end-to-end rustle-plan → rustle-deploy workflow
- Maintain full feature compatibility with existing specs
- Support complex playbooks with 100+ hosts and 1000+ tasks

## API/Interface Design

### Rustle Plan Compatible Data Structures

```rust
/// Rustle-plan compatible execution plan format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustlePlanOutput {
    pub metadata: RustlePlanMetadata,
    pub plays: Vec<PlayPlan>,
    pub binary_deployments: Vec<BinaryDeploymentPlan>,
    pub total_tasks: u32,
    pub estimated_duration: Option<Duration>,
    pub estimated_compilation_time: Option<Duration>,
    pub parallelism_score: f32,
    pub network_efficiency_score: f32,
    pub hosts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustlePlanMetadata {
    pub created_at: DateTime<Utc>,
    pub rustle_version: String,
    pub playbook_hash: String,
    pub inventory_hash: String,
    pub planning_options: PlanningOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningOptions {
    pub limit: Option<String>,
    pub tags: Vec<String>,
    pub skip_tags: Vec<String>,
    pub check_mode: bool,
    pub diff_mode: bool,
    pub forks: u32,
    pub serial: Option<u32>,
    pub strategy: ExecutionStrategy,
    pub binary_threshold: u32,
    pub force_binary: bool,
    pub force_ssh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStrategy {
    Linear,
    Free,
    BinaryHybrid,
    BinaryOnly,
    SshOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayPlan {
    pub play_id: String,
    pub name: String,
    pub strategy: ExecutionStrategy,
    pub serial: Option<u32>,
    pub hosts: Vec<String>,
    pub batches: Vec<TaskBatch>,
    pub handlers: Vec<HandlerDefinition>,
    pub estimated_duration: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBatch {
    pub batch_id: String,
    pub hosts: Vec<String>,
    pub tasks: Vec<TaskPlan>,
    pub parallel_groups: Vec<ParallelGroup>,
    pub dependencies: Vec<String>,
    pub estimated_duration: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    pub task_id: String,
    pub name: String,
    pub module: String,
    pub args: HashMap<String, serde_json::Value>,
    pub hosts: Vec<String>,
    pub dependencies: Vec<String>,
    pub conditions: Vec<TaskCondition>,
    pub tags: Vec<String>,
    pub notify: Vec<String>,
    pub execution_order: u32,
    pub can_run_parallel: bool,
    pub estimated_duration: Duration,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskCondition {
    Tag { tags: Vec<String> },
    When { expression: String },
    Skip { condition: String },
    Only { hosts: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelGroup {
    pub group_id: String,
    pub tasks: Vec<String>,
    pub max_parallelism: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerDefinition {
    pub handler_id: String,
    pub name: String,
    pub module: String,
    pub args: HashMap<String, serde_json::Value>,
    pub conditions: Vec<TaskCondition>,
    pub execution_order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryDeploymentPlan {
    pub deployment_id: String,
    pub target_hosts: Vec<String>,
    pub target_architecture: String,
    pub task_ids: Vec<String>,
    pub estimated_savings: Duration,
    pub compilation_requirements: CompilationRequirements,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationRequirements {
    pub modules: Vec<String>,
    pub static_files: Vec<String>,
    pub target_triple: String,
    pub optimization_level: String,
    pub features: Vec<String>,
}
```

### Parser Implementation

```rust
pub struct RustlePlanParser {
    schema_validator: SchemaValidator,
    binary_analyzer: BinaryDeploymentAnalyzer,
}

impl RustlePlanParser {
    pub fn new() -> Self;
    
    pub fn parse_rustle_plan_output(
        &self, 
        content: &str
    ) -> Result<RustlePlanOutput, RustlePlanParseError>;
    
    pub fn convert_to_execution_plan(
        &self,
        rustle_plan: &RustlePlanOutput,
    ) -> Result<ExecutionPlan, ConversionError>;
    
    pub fn extract_binary_deployments(
        &self,
        rustle_plan: &RustlePlanOutput,
    ) -> Result<Vec<BinaryDeployment>, ExtractionError>;
    
    pub fn analyze_deployment_opportunities(
        &self,
        tasks: &[TaskPlan],
        hosts: &[String],
        threshold: u32,
    ) -> Result<Vec<BinaryDeploymentPlan>, AnalysisError>;
    
    pub fn validate_rustle_plan(
        &self,
        plan: &RustlePlanOutput,
    ) -> Result<(), ValidationError>;
}

pub struct BinaryDeploymentAnalyzer {
    module_registry: ModuleRegistry,
    architecture_detector: ArchitectureDetector,
}

impl BinaryDeploymentAnalyzer {
    pub fn analyze_tasks_for_binary_deployment(
        &self,
        tasks: &[TaskPlan],
        hosts: &[String],
        threshold: u32,
    ) -> Result<Vec<BinaryDeploymentPlan>, AnalysisError>;
    
    pub fn estimate_compilation_time(
        &self,
        deployment: &BinaryDeploymentPlan,
    ) -> Result<Duration, EstimationError>;
    
    pub fn calculate_network_savings(
        &self,
        tasks: &[TaskPlan],
        deployment_method: &str,
    ) -> Result<f32, CalculationError>;
    
    pub fn assess_binary_compatibility(
        &self,
        task: &TaskPlan,
    ) -> Result<BinaryCompatibility, AssessmentError>;
}

#[derive(Debug, Clone)]
pub enum BinaryCompatibility {
    FullyCompatible,
    PartiallyCompatible { limitations: Vec<String> },
    Incompatible { reasons: Vec<String> },
}
```

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum RustlePlanParseError {
    #[error("Invalid JSON format: {reason}")]
    InvalidJson { reason: String },
    
    #[error("Missing required field: {field}")]
    MissingField { field: String },
    
    #[error("Invalid field format: {field} expected {expected}, got {actual}")]
    InvalidFieldFormat { field: String, expected: String, actual: String },
    
    #[error("Schema validation failed: {errors:?}")]
    SchemaValidation { errors: Vec<String> },
    
    #[error("Unsupported rustle-plan version: {version}")]
    UnsupportedVersion { version: String },
    
    #[error("Malformed task definition: {task_id} - {reason}")]
    MalformedTask { task_id: String, reason: String },
    
    #[error("Invalid execution strategy: {strategy}")]
    InvalidStrategy { strategy: String },
}

#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    #[error("Failed to convert play structure: {play_id} - {reason}")]
    PlayConversion { play_id: String, reason: String },
    
    #[error("Task conversion failed: {task_id} - {reason}")]
    TaskConversion { task_id: String, reason: String },
    
    #[error("Missing inventory information for host: {host}")]
    MissingInventory { host: String },
    
    #[error("Binary deployment extraction failed: {reason}")]
    BinaryDeploymentExtraction { reason: String },
}

#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error("Binary compatibility analysis failed for task {task_id}: {reason}")]
    CompatibilityAnalysis { task_id: String, reason: String },
    
    #[error("Architecture detection failed for hosts: {hosts:?}")]
    ArchitectureDetection { hosts: Vec<String> },
    
    #[error("Module dependency resolution failed: {module} - {reason}")]
    ModuleDependency { module: String, reason: String },
    
    #[error("Network efficiency calculation failed: {reason}")]
    NetworkEfficiency { reason: String },
}
```

## File and Package Structure

```
src/execution/
├── mod.rs                     # Module exports (update existing)
├── rustle_plan.rs             # RustlePlanOutput parsing
├── compatibility.rs           # Format conversion logic
├── binary_analyzer.rs         # Binary deployment analysis
├── plan_converter.rs          # Convert rustle-plan to ExecutionPlan
└── validation.rs              # Rustle-plan specific validation

src/binary/
├── mod.rs                     # Binary deployment module
├── analyzer.rs                # Binary compatibility analysis
├── deployment_planner.rs      # Binary deployment planning
├── architecture_detector.rs   # Target architecture detection
└── module_registry.rs         # Module compatibility registry

tests/execution/
├── rustle_plan_tests.rs       # Rustle-plan parsing tests
├── compatibility_tests.rs     # Format conversion tests
├── binary_analysis_tests.rs   # Binary deployment analysis tests
└── fixtures/
    ├── rustle_plan_outputs/    # Real rustle-plan JSON outputs
    ├── complex_plans/          # Multi-play, multi-host plans
    └── binary_scenarios/       # Binary deployment test cases
```

## Implementation Details

### Phase 1: Core Parsing Support
1. Implement RustlePlanOutput data structures
2. Add JSON parsing for actual rustle-plan format
3. Create basic validation for required fields
4. Update CLI to accept rustle-plan JSON format

### Phase 2: Format Conversion
1. Implement conversion from RustlePlanOutput to ExecutionPlan
2. Add backward compatibility support
3. Create binary deployment extraction logic
4. Integrate with existing DeploymentManager

### Phase 3: Binary Analysis
1. Implement binary compatibility analysis
2. Add architecture detection from inventory
3. Create module dependency resolution
4. Add network efficiency calculations

### Phase 4: Advanced Features
1. Add conditional execution support
2. Implement handler execution planning
3. Create parallel task grouping
4. Add performance optimization analysis

### Key Algorithms

**Binary Deployment Analysis**:
```rust
impl BinaryDeploymentAnalyzer {
    pub fn analyze_tasks_for_binary_deployment(
        &self,
        tasks: &[TaskPlan],
        hosts: &[String],
        threshold: u32,
    ) -> Result<Vec<BinaryDeploymentPlan>, AnalysisError> {
        let mut deployments = Vec::new();
        
        // Group tasks by compatibility and architecture
        let compatibility_groups = self.group_tasks_by_compatibility(tasks)?;
        
        for (architecture, compatible_tasks) in compatibility_groups {
            // Only create binary deployment if we have enough tasks
            if compatible_tasks.len() >= threshold as usize {
                let deployment = BinaryDeploymentPlan {
                    deployment_id: format!("binary-{}", uuid::Uuid::new_v4()),
                    target_hosts: hosts.clone(),
                    target_architecture: architecture.clone(),
                    task_ids: compatible_tasks.iter().map(|t| t.task_id.clone()).collect(),
                    estimated_savings: self.calculate_time_savings(&compatible_tasks)?,
                    compilation_requirements: CompilationRequirements {
                        modules: self.extract_required_modules(&compatible_tasks),
                        static_files: self.extract_static_files(&compatible_tasks),
                        target_triple: architecture,
                        optimization_level: "release".to_string(),
                        features: vec!["binary-deployment".to_string()],
                    },
                };
                
                deployments.push(deployment);
            }
        }
        
        Ok(deployments)
    }
    
    fn group_tasks_by_compatibility(
        &self,
        tasks: &[TaskPlan],
    ) -> Result<HashMap<String, Vec<TaskPlan>>, AnalysisError> {
        let mut groups = HashMap::new();
        
        for task in tasks {
            let compatibility = self.assess_binary_compatibility(task)?;
            
            match compatibility {
                BinaryCompatibility::FullyCompatible => {
                    let arch = self.detect_target_architecture(&task.hosts)?;
                    groups.entry(arch).or_insert_with(Vec::new).push(task.clone());
                }
                BinaryCompatibility::PartiallyCompatible { limitations } => {
                    // Log limitations but still include if major functionality works
                    if !limitations.iter().any(|l| l.contains("critical")) {
                        let arch = self.detect_target_architecture(&task.hosts)?;
                        groups.entry(arch).or_insert_with(Vec::new).push(task.clone());
                    }
                }
                BinaryCompatibility::Incompatible { .. } => {
                    // Skip incompatible tasks
                    continue;
                }
            }
        }
        
        Ok(groups)
    }
}
```

**Rustle Plan Format Conversion**:
```rust
impl RustlePlanParser {
    pub fn convert_to_execution_plan(
        &self,
        rustle_plan: &RustlePlanOutput,
    ) -> Result<ExecutionPlan, ConversionError> {
        let mut tasks = Vec::new();
        
        // Convert play-based structure to flat task list
        for play in &rustle_plan.plays {
            for batch in &play.batches {
                for task in &batch.tasks {
                    tasks.push(Task {
                        id: task.task_id.clone(),
                        name: task.name.clone(),
                        task_type: self.convert_module_to_task_type(&task.module)?,
                        module: task.module.clone(),
                        args: task.args.clone(),
                        dependencies: task.dependencies.clone(),
                        conditions: self.convert_conditions(&task.conditions)?,
                        target_hosts: TargetSelector::Hosts(task.hosts.clone()),
                        timeout: Some(task.estimated_duration),
                        retry_policy: None, // Extract from args if present
                        failure_policy: self.determine_failure_policy(&task.risk_level),
                    });
                }
            }
        }
        
        Ok(ExecutionPlan {
            metadata: ExecutionPlanMetadata {
                version: "1.0".to_string(),
                created_at: rustle_plan.metadata.created_at,
                rustle_plan_version: rustle_plan.metadata.rustle_version.clone(),
                plan_id: format!("rustle-{}", rustle_plan.metadata.playbook_hash),
                description: None,
                author: None,
                tags: rustle_plan.metadata.planning_options.tags.clone(),
            },
            tasks,
            inventory: self.construct_inventory_spec(&rustle_plan.hosts)?,
            strategy: rustle_plan.metadata.planning_options.strategy.clone(),
            facts_template: FactsTemplate {
                global_facts: vec!["ansible_facts".to_string()],
                host_facts: vec!["ansible_hostname".to_string()],
                custom_facts: HashMap::new(),
            },
            deployment_config: DeploymentConfig::default(),
            modules: self.extract_module_specs(rustle_plan)?,
        })
    }
}
```

## Testing Strategy

### Unit Tests
- **Parser Tests**: Valid/invalid rustle-plan JSON parsing
- **Conversion Tests**: RustlePlanOutput → ExecutionPlan conversion
- **Binary Analysis Tests**: Task compatibility assessment
- **Architecture Detection Tests**: Target platform identification

### Integration Tests
- **End-to-end**: Complete rustle-plan → rustle-deploy workflow
- **Real Data**: Tests with actual rustle-plan outputs
- **Binary Deployment**: Full binary compilation and deployment
- **Error Handling**: Comprehensive error scenario coverage

### Test Data
```
tests/fixtures/rustle_plan_outputs/
├── simple/
│   ├── single_play.json        # Basic single play output
│   ├── debug_only.json         # Debug-only tasks
│   └── localhost_only.json     # Local execution only
├── complex/
│   ├── multi_play.json         # Multiple plays with dependencies
│   ├── binary_hybrid.json      # Mixed binary/SSH execution
│   ├── conditional_tasks.json  # Tasks with when conditions
│   └── handlers.json           # Tasks with handler notifications
├── binary_deployment/
│   ├── high_threshold.json     # Many compatible tasks
│   ├── mixed_compatibility.json # Some compatible, some not
│   └── multi_arch.json         # Multiple target architectures
└── edge_cases/
    ├── empty_plays.json        # Plays with no tasks
    ├── malformed_tasks.json    # Invalid task definitions
    └── missing_fields.json     # Required fields missing
```

## Edge Cases & Error Handling

### Parsing Edge Cases
- Malformed JSON from rustle-plan
- Missing required fields in task definitions
- Invalid execution strategies
- Circular handler dependencies
- Empty plays or batches

### Conversion Edge Cases
- Tasks with no target hosts
- Conflicting execution strategies
- Invalid module names
- Missing task dependencies
- Incompatible binary deployment configurations

### Binary Analysis Edge Cases
- No compatible tasks for binary deployment
- Mixed architectures in host groups
- Modules that can't be statically linked
- Tasks requiring interactive input
- Platform-specific module dependencies

### Recovery Strategies
- Graceful fallback to SSH execution for incompatible tasks
- Partial binary deployment with remaining SSH tasks
- Default architecture selection when detection fails
- Warning collection for non-critical validation failures
- Automatic retry for transient parsing errors

## Dependencies

### External Crates
```toml
[dependencies]
# Existing dependencies...
uuid = { version = "1", features = ["v4"] }
regex = "1.10"              # Pattern matching for architecture detection
semver = "1.0"              # Version compatibility checking
url = "2.4"                 # URL parsing for module sources
```

### Internal Dependencies
- `rustle_deploy::execution` - Existing execution plan types
- `rustle_deploy::deploy` - Deployment manager integration
- `rustle_deploy::types` - Core type definitions

## Configuration

### Parser Configuration
```toml
[rustle_plan]
strict_parsing = false          # Allow unknown fields
validate_on_parse = true        # Validate structure during parsing
auto_convert_format = true      # Automatically convert to ExecutionPlan
binary_threshold = 5            # Minimum tasks for binary deployment

[binary_analysis]
enable_compatibility_check = true
default_architecture = "x86_64-unknown-linux-gnu"
force_static_linking = true
optimization_level = "release"
```

### Environment Variables
- `RUSTLE_PLAN_STRICT_MODE`: Enable strict parsing validation
- `RUSTLE_BINARY_THRESHOLD`: Override binary deployment threshold
- `RUSTLE_DEFAULT_ARCH`: Default target architecture
- `RUSTLE_MODULE_REGISTRY_PATH`: Custom module registry location

## Documentation

### Usage Examples
```rust
// Parse rustle-plan output
let content = std::fs::read_to_string("rustle_plan_output.json")?;
let parser = RustlePlanParser::new();
let rustle_plan = parser.parse_rustle_plan_output(&content)?;

// Analyze for binary deployment opportunities
let binary_deployments = parser.extract_binary_deployments(&rustle_plan)?;

// Convert to standard ExecutionPlan format
let execution_plan = parser.convert_to_execution_plan(&rustle_plan)?;

// Create deployment plan
let deployment_manager = DeploymentManager::new(config);
let deployment_plan = deployment_manager.create_deployment_plan_from_rustle_plan(
    &rustle_plan,
    &binary_deployments,
).await?;
```

### CLI Integration
```bash
# Process rustle-plan output directly
rustle-deploy rustle_plan_output.json --auto-binary

# Force binary deployment
rustle-deploy rustle_plan_output.json --force-binary --threshold 3

# SSH-only execution
rustle-deploy rustle_plan_output.json --force-ssh
```

## Integration Points

### DeploymentManager Integration
```rust
impl DeploymentManager {
    pub async fn create_deployment_plan_from_rustle_plan(
        &self,
        rustle_plan: &RustlePlanOutput,
        binary_deployments: &[BinaryDeploymentPlan],
    ) -> Result<DeploymentPlan, DeployError>;
    
    pub async fn execute_rustle_plan(
        &self,
        rustle_plan: &RustlePlanOutput,
    ) -> Result<ExecutionReport, DeployError>;
}
```

### CLI Integration
Update `rustle-deploy` CLI to detect and handle rustle-plan JSON format automatically while maintaining backward compatibility with existing ExecutionPlan format.