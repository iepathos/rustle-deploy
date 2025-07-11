# Spec 020: Execution Plan Integration

## Feature Summary

Implement comprehensive execution plan parsing, validation, and integration to enable rustle-deploy to process output from rustle-plan. This foundational component defines the data structures, parsing logic, and validation required to bridge the gap between planning and deployment phases.

**Problem it solves**: rustle-deploy currently has placeholder execution plan handling and cannot process real rustle-plan output, making the deployment pipeline non-functional.

**High-level approach**: Define complete execution plan schema, implement robust parsing and validation, and integrate with existing deployment workflows.

## Goals & Requirements

### Functional Requirements
- Parse rustle-plan JSON/YAML output into structured execution plans
- Validate execution plan schema and dependencies
- Extract binary deployment requirements from execution plans
- Support multiple execution plan formats (JSON, YAML)
- Handle execution plan versioning and compatibility
- Extract task dependencies and execution ordering
- Support template variables and facts integration
- Generate deployment targets from execution plan inventory
- Validate module requirements and dependencies
- Support conditional execution and strategy selection

### Non-functional Requirements
- **Performance**: Parse execution plans with 1000+ tasks in <100ms
- **Reliability**: 99.9%+ parsing success rate for valid plans
- **Compatibility**: Support rustle-plan v1.0+ output formats
- **Memory**: Efficient parsing for plans up to 100MB
- **Validation**: Comprehensive error reporting for invalid plans

### Success Criteria
- Successfully parse all valid rustle-plan output formats
- Provide clear error messages for parsing failures
- Extract complete deployment information from execution plans
- Support template variable substitution
- Enable end-to-end execution plan processing

## API/Interface Design

### Core Data Structures

```rust
/// Complete execution plan from rustle-plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub metadata: ExecutionPlanMetadata,
    pub tasks: Vec<Task>,
    pub inventory: InventorySpec,
    pub strategy: ExecutionStrategy,
    pub facts_template: FactsTemplate,
    pub deployment_config: DeploymentConfig,
    pub modules: Vec<ModuleSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlanMetadata {
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub rustle_plan_version: String,
    pub plan_id: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub task_type: TaskType,
    pub module: String,
    pub args: HashMap<String, serde_json::Value>,
    pub dependencies: Vec<String>,
    pub conditions: Vec<Condition>,
    pub target_hosts: TargetSelector,
    pub timeout: Option<Duration>,
    pub retry_policy: Option<RetryPolicy>,
    pub failure_policy: FailurePolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    Command,
    Copy,
    Template,
    Package,
    Service,
    Custom { module_name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub variable: String,
    pub operator: ConditionOperator,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    Contains,
    StartsWith,
    EndsWith,
    GreaterThan,
    LessThan,
    Exists,
    NotExists,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetSelector {
    All,
    Groups(Vec<String>),
    Hosts(Vec<String>),
    Expression(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub delay: Duration,
    pub backoff: BackoffStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Fixed,
    Linear,
    Exponential,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FailurePolicy {
    Abort,
    Continue,
    Rollback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventorySpec {
    pub format: InventoryFormat,
    pub source: InventorySource,
    pub groups: HashMap<String, HostGroup>,
    pub hosts: HashMap<String, Host>,
    pub variables: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InventoryFormat {
    Yaml,
    Json,
    Ini,
    Dynamic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InventorySource {
    Inline { content: String },
    File { path: String },
    Url { url: String },
    Dynamic { script: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostGroup {
    pub hosts: Vec<String>,
    pub variables: HashMap<String, serde_json::Value>,
    pub children: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    pub address: String,
    pub connection: ConnectionConfig,
    pub variables: HashMap<String, serde_json::Value>,
    pub target_triple: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub method: ConnectionMethod,
    pub username: Option<String>,
    pub password: Option<String>,
    pub key_file: Option<String>,
    pub port: Option<u16>,
    pub timeout: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionMethod {
    Ssh,
    WinRm,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactsTemplate {
    pub global_facts: Vec<String>,
    pub host_facts: Vec<String>,
    pub custom_facts: HashMap<String, FactDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactDefinition {
    pub command: String,
    pub parser: FactParser,
    pub cache_ttl: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FactParser {
    Json,
    Yaml,
    Text,
    Regex { pattern: String },
    Custom { script: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSpec {
    pub name: String,
    pub source: ModuleSource,
    pub version: Option<String>,
    pub checksum: Option<String>,
    pub dependencies: Vec<String>,
    pub static_link: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleSource {
    Builtin,
    File { path: String },
    Git { repository: String, reference: String },
    Http { url: String },
    Registry { name: String, version: String },
}
```

### Parser API

```rust
pub struct ExecutionPlanParser {
    schema_validator: SchemaValidator,
    template_processor: TemplateProcessor,
}

impl ExecutionPlanParser {
    pub fn new() -> Self;
    
    pub fn parse(&self, content: &str, format: PlanFormat) -> Result<ExecutionPlan, ParseError>;
    
    pub fn validate(&self, plan: &ExecutionPlan) -> Result<(), ValidationError>;
    
    pub fn resolve_templates(
        &self, 
        plan: &ExecutionPlan, 
        variables: &HashMap<String, serde_json::Value>
    ) -> Result<ExecutionPlan, TemplateError>;
    
    pub fn extract_deployment_targets(
        &self, 
        plan: &ExecutionPlan
    ) -> Result<Vec<DeploymentTarget>, ExtractionError>;
    
    pub fn validate_dependencies(&self, plan: &ExecutionPlan) -> Result<(), DependencyError>;
    
    pub fn compute_execution_order(&self, plan: &ExecutionPlan) -> Result<Vec<String>, OrderingError>;
}

#[derive(Debug, Clone)]
pub enum PlanFormat {
    Json,
    Yaml,
    Auto,
}

pub struct SchemaValidator {
    json_schema: serde_json::Value,
}

impl SchemaValidator {
    pub fn validate_plan(&self, plan: &ExecutionPlan) -> Result<(), ValidationError>;
    pub fn validate_task(&self, task: &Task) -> Result<(), ValidationError>;
    pub fn validate_inventory(&self, inventory: &InventorySpec) -> Result<(), ValidationError>;
}

pub struct TemplateProcessor {
    engine: TemplateEngine,
}

impl TemplateProcessor {
    pub fn process_plan(
        &self,
        plan: &ExecutionPlan,
        variables: &HashMap<String, serde_json::Value>
    ) -> Result<ExecutionPlan, TemplateError>;
    
    pub fn process_task_args(
        &self,
        args: &HashMap<String, serde_json::Value>,
        variables: &HashMap<String, serde_json::Value>
    ) -> Result<HashMap<String, serde_json::Value>, TemplateError>;
}
```

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Invalid JSON format: {reason}")]
    InvalidJson { reason: String },
    
    #[error("Invalid YAML format: {reason}")]
    InvalidYaml { reason: String },
    
    #[error("Schema validation failed: {errors:?}")]
    SchemaValidation { errors: Vec<String> },
    
    #[error("Unknown plan format")]
    UnknownFormat,
    
    #[error("Missing required field: {field}")]
    MissingField { field: String },
    
    #[error("Invalid field value: {field} = {value}")]
    InvalidFieldValue { field: String, value: String },
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Circular dependency detected: {cycle:?}")]
    CircularDependency { cycle: Vec<String> },
    
    #[error("Missing task dependency: {task} -> {dependency}")]
    MissingDependency { task: String, dependency: String },
    
    #[error("Invalid target selector: {selector}")]
    InvalidTargetSelector { selector: String },
    
    #[error("Unknown module: {module}")]
    UnknownModule { module: String },
    
    #[error("Invalid inventory format: {reason}")]
    InvalidInventory { reason: String },
}

#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error("Template processing failed: {reason}")]
    ProcessingFailed { reason: String },
    
    #[error("Missing template variable: {variable}")]
    MissingVariable { variable: String },
    
    #[error("Invalid template syntax: {syntax}")]
    InvalidSyntax { syntax: String },
}
```

## File and Package Structure

```
src/execution/
├── mod.rs                     # Module exports
├── plan.rs                    # ExecutionPlan and related types
├── parser.rs                  # ExecutionPlanParser implementation
├── validator.rs               # Schema validation
├── template.rs                # Template processing
├── inventory.rs               # Inventory parsing and processing
├── dependency.rs              # Dependency resolution
├── extractor.rs               # DeploymentTarget extraction
└── error.rs                   # Error types

src/types/
├── execution.rs               # Core execution types (add to existing)

tests/execution/
├── parser_tests.rs
├── validator_tests.rs
├── template_tests.rs
├── integration_tests.rs
└── fixtures/                  # Test execution plans
    ├── simple_plan.json
    ├── complex_plan.yaml
    ├── template_plan.json
    └── invalid_plans/
```

## Implementation Details

### Phase 1: Basic Parsing
1. Implement core ExecutionPlan data structures
2. Add JSON/YAML parsing capabilities
3. Create basic schema validation
4. Integrate with existing DeploymentManager

### Phase 2: Advanced Validation
1. Implement dependency cycle detection
2. Add inventory parsing and validation
3. Create target extraction logic
4. Add comprehensive error handling

### Phase 3: Template Processing
1. Implement template variable substitution
2. Add conditional execution logic
3. Create dynamic inventory support
4. Add facts integration placeholders

### Key Algorithms

**Dependency Resolution**:
```rust
fn resolve_dependencies(tasks: &[Task]) -> Result<Vec<String>, ValidationError> {
    let mut graph = HashMap::new();
    let mut in_degree = HashMap::new();
    
    // Build dependency graph
    for task in tasks {
        graph.insert(task.id.clone(), task.dependencies.clone());
        in_degree.insert(task.id.clone(), task.dependencies.len());
    }
    
    // Topological sort
    let mut queue = VecDeque::new();
    let mut result = Vec::new();
    
    // Find tasks with no dependencies
    for (task_id, degree) in &in_degree {
        if *degree == 0 {
            queue.push_back(task_id.clone());
        }
    }
    
    while let Some(task_id) = queue.pop_front() {
        result.push(task_id.clone());
        
        // Update dependencies
        for (dependent_id, dependencies) in &graph {
            if dependencies.contains(&task_id) {
                let current_degree = in_degree.get_mut(dependent_id).unwrap();
                *current_degree -= 1;
                if *current_degree == 0 {
                    queue.push_back(dependent_id.clone());
                }
            }
        }
    }
    
    // Check for cycles
    if result.len() != tasks.len() {
        return Err(ValidationError::CircularDependency {
            cycle: find_cycle(&graph),
        });
    }
    
    Ok(result)
}
```

**Target Extraction**:
```rust
impl ExecutionPlanParser {
    pub fn extract_deployment_targets(
        &self,
        plan: &ExecutionPlan,
    ) -> Result<Vec<DeploymentTarget>, ExtractionError> {
        let mut targets = Vec::new();
        let inventory = self.parse_inventory(&plan.inventory)?;
        
        // Group tasks by target architecture
        let mut arch_groups = HashMap::new();
        
        for host in inventory.hosts.values() {
            let target_triple = host.target_triple
                .clone()
                .unwrap_or_else(|| "x86_64-unknown-linux-gnu".to_string());
            
            arch_groups
                .entry(target_triple.clone())
                .or_insert_with(Vec::new)
                .push(host.address.clone());
        }
        
        // Create deployment targets for each architecture
        for (target_triple, hosts) in arch_groups {
            for host in hosts {
                targets.push(DeploymentTarget {
                    host,
                    target_path: "/tmp/rustle-runner".to_string(),
                    binary_compilation_id: format!("rustle-{}", target_triple),
                    deployment_method: DeploymentMethod::Ssh,
                    status: DeploymentStatus::Pending,
                    deployed_at: None,
                    version: "1.0.0".to_string(),
                });
            }
        }
        
        Ok(targets)
    }
}
```

## Testing Strategy

### Unit Tests
- **Parser Tests**: Valid/invalid execution plans, format detection
- **Validator Tests**: Schema validation, dependency cycles, inventory validation
- **Template Tests**: Variable substitution, conditional logic
- **Extractor Tests**: Target generation, architecture detection

### Integration Tests
- **End-to-end**: Complete parsing and validation workflow
- **Format Support**: JSON/YAML parsing consistency
- **Error Handling**: Comprehensive error scenario coverage
- **Performance**: Large execution plan parsing benchmarks

### Test Data
```
tests/fixtures/execution_plans/
├── basic/
│   ├── simple_plan.json        # Minimal valid plan
│   ├── single_task.yaml        # Single task execution
│   └── no_dependencies.json    # Tasks without dependencies
├── complex/
│   ├── multi_host.yaml         # Multi-host deployment
│   ├── with_templates.json     # Template variables
│   ├── conditional_tasks.yaml  # Conditional execution
│   └── large_plan.json         # 1000+ tasks
├── invalid/
│   ├── circular_deps.json      # Circular dependencies
│   ├── missing_fields.yaml     # Required field validation
│   ├── invalid_syntax.json     # Malformed JSON/YAML
│   └── unknown_modules.yaml    # Non-existent modules
└── integration/
    ├── rustle_plan_output.json  # Real rustle-plan output
    └── ansible_converted.yaml   # Converted Ansible playbook
```

## Edge Cases & Error Handling

### Parsing Edge Cases
- Malformed JSON/YAML input
- Mixed format detection (JSON with .yaml extension)
- Large execution plans (>100MB)
- Unicode characters in task names and descriptions
- Nested template variables

### Validation Edge Cases
- Circular dependency chains
- Self-referencing tasks
- Missing inventory hosts
- Invalid target selectors
- Conflicting task requirements

### Template Edge Cases
- Undefined template variables
- Recursive template expansion
- Template syntax errors
- Type conversion failures

### Recovery Strategies
- Graceful degradation for non-critical validation errors
- Partial parsing with warning collection
- Default value substitution for missing variables
- Fallback target architecture detection

## Dependencies

### External Crates
```toml
[dependencies]
serde_yaml = "0.9"
jsonschema = "0.18"
handlebars = "4.5"
petgraph = "0.6"        # Dependency graph processing
url = "2.4"             # URL parsing for inventory sources
regex = "1.10"          # Pattern matching and validation
```

### Internal Dependencies
- `rustle_deploy::types` - Existing type definitions
- `rustle_deploy::deploy` - Integration with deployment manager
- `rustle_deploy::error` - Error handling patterns

## Configuration

### Schema Configuration
```toml
[execution]
schema_validation = true
strict_mode = false
max_plan_size_mb = 100
template_engine = "handlebars"

[parsing]
auto_detect_format = true
validate_on_parse = true
allow_unknown_fields = false

[templates]
variable_prefix = "{{"
variable_suffix = "}}"
strict_variables = true
```

### Environment Variables
- `RUSTLE_EXECUTION_SCHEMA_PATH`: Custom schema file location
- `RUSTLE_TEMPLATE_STRICT`: Enable strict template validation
- `RUSTLE_MAX_PLAN_SIZE`: Maximum execution plan size
- `RUSTLE_PARSE_TIMEOUT`: Parsing timeout in seconds

## Documentation

### API Documentation
```rust
/// Parse an execution plan from rustle-plan output
/// 
/// # Arguments
/// * `content` - JSON or YAML content from rustle-plan
/// * `format` - Expected format (Json, Yaml, or Auto-detect)
/// 
/// # Returns
/// * `Ok(ExecutionPlan)` - Successfully parsed execution plan
/// * `Err(ParseError)` - Parsing or validation failure
/// 
/// # Examples
/// ```rust
/// let content = std::fs::read_to_string("plan.json")?;
/// let parser = ExecutionPlanParser::new();
/// let plan = parser.parse(&content, PlanFormat::Auto)?;
/// ```
```

### Usage Examples
```rust
// Basic parsing
let parser = ExecutionPlanParser::new();
let plan = parser.parse(&rustle_plan_output, PlanFormat::Auto)?;

// With template variables
let variables = hashmap! {
    "environment".to_string() => json!("production"),
    "version".to_string() => json!("1.2.3"),
};
let resolved_plan = parser.resolve_templates(&plan, &variables)?;

// Extract deployment targets
let targets = parser.extract_deployment_targets(&resolved_plan)?;

// Integration with deployment manager
let deployment_plan = manager.create_deployment_plan_from_execution(
    &resolved_plan,
    &targets,
).await?;
```

## Integration Points

### DeploymentManager Integration
```rust
impl DeploymentManager {
    pub async fn create_deployment_plan_from_execution(
        &self,
        execution_plan: &ExecutionPlan,
        targets: &[DeploymentTarget],
    ) -> Result<DeploymentPlan, DeployError> {
        // Replace existing create_deployment_plan method
        // to work with structured ExecutionPlan instead of JSON string
    }
}
```

### CLI Integration
```rust
// Update rustle-deploy CLI to accept execution plans
rustle-deploy execute execution-plan.json --inventory hosts.yml
rustle-deploy compile execution-plan.yaml --target x86_64-unknown-linux-gnu
```