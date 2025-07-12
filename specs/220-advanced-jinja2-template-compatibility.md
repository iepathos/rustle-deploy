# Spec 220: Advanced Jinja2 Template Compatibility

## Feature Summary

Enhance the template module to provide comprehensive Jinja2 compatibility by implementing advanced template control structures, including conditionals, loops, and complex variable access patterns. This addresses the remaining 2 test failures from specification 210 and provides full Jinja2-to-Handlebars template conversion capabilities.

**Architecture Note**: This specification builds on the basic Jinja2 compatibility layer introduced in spec 210, adding sophisticated template parsing and conversion for complex control structures while maintaining the Handlebars-based rendering engine.

## Goals & Requirements

### Functional Requirements
- **Conditional template blocks**: Support for `{% if %}`, `{% else %}`, `{% elif %}`, and `{% endif %}` constructs
- **Loop template blocks**: Support for `{% for item in array %}` and `{% endfor %}` with nested object access
- **Complex variable access**: Support for dot notation like `{{ server.name }}` and `{{ item.property }}`
- **Comparison operations**: Support for equality, inequality, and boolean logic in template conditions
- **Nested template structures**: Proper handling of nested conditionals and loops
- **Template whitespace control**: Preserve formatting and indentation in generated output

### Non-Functional Requirements
- Maintain backward compatibility with existing simple template syntax
- Preserve performance characteristics of template rendering
- Ensure robust error handling for malformed template syntax
- Support complex nested data structures with arrays and objects
- Maintain security best practices preventing template injection attacks

### Success Criteria
- All 72 tests in the file operations test suite pass (100% pass rate)
- Complex template rendering matches expected Jinja2 output exactly
- Template conversion handles nested control structures correctly
- Performance regression testing shows no significant impact
- Error messages provide clear feedback for template syntax issues

## API/Interface Design

### Enhanced Template Processor Interface
```rust
pub struct AdvancedTemplateProcessor {
    handlebars: Handlebars<'static>,
    jinja_parser: Jinja2Parser,
}

impl AdvancedTemplateProcessor {
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(false);
        
        // Register advanced helpers
        handlebars.register_helper("default", Box::new(default_helper));
        handlebars.register_helper("quote", Box::new(quote_helper));
        handlebars.register_helper("eq", Box::new(equality_helper));
        
        Self { 
            handlebars,
            jinja_parser: Jinja2Parser::new(),
        }
    }
    
    pub fn render_template(
        &self,
        template_content: &str,
        variables: &serde_json::Value,
    ) -> Result<String, TemplateError>;
}
```

### Jinja2 Parser Interface
```rust
pub struct Jinja2Parser {
    conditional_regex: Regex,
    loop_regex: Regex,
    variable_regex: Regex,
}

impl Jinja2Parser {
    pub fn new() -> Self;
    pub fn convert_to_handlebars(&self, jinja_template: &str) -> Result<String, ParseError>;
    
    fn convert_conditionals(&self, template: &str) -> Result<String, ParseError>;
    fn convert_loops(&self, template: &str) -> Result<String, ParseError>;
    fn convert_variables(&self, template: &str) -> Result<String, ParseError>;
    fn handle_nested_structures(&self, template: &str) -> Result<String, ParseError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Invalid conditional syntax: {syntax}")]
    InvalidConditional { syntax: String },
    
    #[error("Unclosed template block: {block_type}")]
    UnclosedBlock { block_type: String },
    
    #[error("Nested structure too deep: {depth}")]
    NestedTooDeep { depth: usize },
    
    #[error("Regex compilation failed: {error}")]
    RegexError { error: String },
}
```

### Template Conversion Result
```rust
#[derive(Debug, Clone)]
pub struct ConversionResult {
    pub handlebars_template: String,
    pub required_helpers: Vec<String>,
    pub variable_paths: Vec<String>,
    pub warnings: Vec<String>,
}
```

## File and Package Structure

### Enhanced Template Module Structure
```
src/modules/files/
├── template.rs                    # Main template module (enhanced)
├── template/
│   ├── mod.rs                    # Template module exports
│   ├── jinja_parser.rs           # Jinja2 parsing and conversion
│   ├── handlebars_helpers.rs     # Custom Handlebars helpers
│   ├── syntax_converter.rs       # Template syntax transformation
│   └── template_processor.rs     # Advanced template processing
└── utils/
    └── template_helpers.rs        # Shared template utilities
```

### Test Enhancement Structure
```
tests/modules/files/
├── template/
│   ├── jinja_conversion_tests.rs # Jinja2 conversion unit tests
│   ├── complex_template_tests.rs # Complex template integration tests
│   └── syntax_edge_cases.rs      # Edge case and error handling tests
└── fixtures/
    ├── complex_templates/         # Complex template test cases
    └── expected_outputs/          # Expected rendering results
```

## Implementation Details

### 1. Jinja2 Conditional Conversion
```rust
impl Jinja2Parser {
    fn convert_conditionals(&self, template: &str) -> Result<String, ParseError> {
        let mut result = template.to_string();
        
        // Convert {% if condition %} to {{#if condition}}
        let if_pattern = Regex::new(r#"\{\%\s*if\s+([^%]+)\s*\%\}"#)?;
        result = if_pattern.replace_all(&result, "{{#if $1}}").to_string();
        
        // Convert {% else %} to {{else}}
        let else_pattern = Regex::new(r#"\{\%\s*else\s*\%\}"#)?;
        result = else_pattern.replace_all(&result, "{{else}}").to_string();
        
        // Convert {% elif condition %} to {{else}}{{#if condition}}
        let elif_pattern = Regex::new(r#"\{\%\s*elif\s+([^%]+)\s*\%\}"#)?;
        result = elif_pattern.replace_all(&result, "{{else}}{{#if $1}}").to_string();
        
        // Convert {% endif %} to {{/if}}
        let endif_pattern = Regex::new(r#"\{\%\s*endif\s*\%\}"#)?;
        result = endif_pattern.replace_all(&result, "{{/if}}").to_string();
        
        Ok(result)
    }
}
```

### 2. Loop Structure Conversion
```rust
impl Jinja2Parser {
    fn convert_loops(&self, template: &str) -> Result<String, ParseError> {
        let mut result = template.to_string();
        
        // Convert {% for item in array %} to {{#each array}}
        let for_pattern = Regex::new(r#"\{\%\s*for\s+(\w+)\s+in\s+(\w+)\s*\%\}"#)?;
        result = for_pattern.replace_all(&result, "{{#each $2}}").to_string();
        
        // Convert {% endfor %} to {{/each}}
        let endfor_pattern = Regex::new(r#"\{\%\s*endfor\s*\%\}"#)?;
        result = endfor_pattern.replace_all(&result, "{{/each}}").to_string();
        
        // Convert loop variable references {{ item.property }} to {{ property }}
        // This requires context-aware replacement within each blocks
        result = self.convert_loop_variables(&result)?;
        
        Ok(result)
    }
    
    fn convert_loop_variables(&self, template: &str) -> Result<String, ParseError> {
        // Complex logic to handle variable scope within loops
        // Convert {{ item.property }} to {{ property }} within {{#each}} blocks
        let mut result = template.to_string();
        
        // Find each block boundaries and convert variables within them
        let each_blocks = self.find_each_blocks(&result)?;
        for block in each_blocks {
            let converted_block = self.convert_variables_in_block(&block)?;
            result = result.replace(&block.original, &converted_block);
        }
        
        Ok(result)
    }
}
```

### 3. Variable Access Enhancement
```rust
impl Jinja2Parser {
    fn convert_variables(&self, template: &str) -> Result<String, ParseError> {
        let mut result = template.to_string();
        
        // Convert dot notation for root level variables
        let dot_notation_pattern = Regex::new(r#"\{\{\s*(\w+)\.(\w+)\s*\}\}"#)?;
        result = dot_notation_pattern.replace_all(&result, "{{$1.$2}}").to_string();
        
        // Handle comparison operations
        let comparison_pattern = Regex::new(r#"\{\{\s*(\w+)\s*(==|!=|<|>|<=|>=)\s*"([^"]+)"\s*\}\}"#)?;
        result = comparison_pattern.replace_all(&result, "{{eq $1 '$3'}}").to_string();
        
        Ok(result)
    }
}
```

### 4. Advanced Handlebars Helpers
```rust
// Equality comparison helper
fn equality_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _rc: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let left = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
    let right = h.param(1).and_then(|v| v.value().as_str()).unwrap_or("");
    
    let result = left == right;
    out.write(&result.to_string())?;
    Ok(())
}

// Enhanced each helper with loop context
fn enhanced_each_helper(
    h: &Helper,
    r: &Handlebars,
    ctx: &Context,
    rc: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let value = h.param(0).ok_or_else(|| {
        RenderError::new("Parameter not found for each helper")
    })?;
    
    if let Some(array) = value.value().as_array() {
        for (index, item) in array.iter().enumerate() {
            let mut local_ctx = ctx.clone();
            local_ctx.insert("index".to_string(), &index);
            local_ctx.insert("first".to_string(), &(index == 0));
            local_ctx.insert("last".to_string(), &(index == array.len() - 1));
            
            if let Some(template) = h.template() {
                template.render(r, &local_ctx, rc, out)?;
            }
        }
    }
    
    Ok(())
}
```

### 5. Error Handling and Validation
```rust
impl AdvancedTemplateProcessor {
    fn validate_template_syntax(&self, template: &str) -> Result<(), TemplateError> {
        // Check for balanced control structures
        self.check_balanced_blocks(template)?;
        
        // Validate variable references
        self.validate_variable_syntax(template)?;
        
        // Check for unsupported Jinja2 features
        self.check_unsupported_features(template)?;
        
        Ok(())
    }
    
    fn check_balanced_blocks(&self, template: &str) -> Result<(), TemplateError> {
        let mut if_stack = 0;
        let mut for_stack = 0;
        
        // Parse through template and track block nesting
        for line in template.lines() {
            if line.contains("{% if ") || line.contains("{%if ") {
                if_stack += 1;
            } else if line.contains("{% endif") {
                if_stack -= 1;
                if if_stack < 0 {
                    return Err(TemplateError::UnbalancedBlocks {
                        block_type: "if".to_string(),
                    });
                }
            }
            // Similar logic for for/endfor blocks
        }
        
        if if_stack != 0 || for_stack != 0 {
            return Err(TemplateError::UnbalancedBlocks {
                block_type: "mixed".to_string(),
            });
        }
        
        Ok(())
    }
}
```

## Testing Strategy

### Unit Test Categories
1. **Jinja2 Conversion Tests**: Verify each syntax transformation works correctly
2. **Template Rendering Tests**: Ensure converted templates produce expected output
3. **Error Handling Tests**: Validate error reporting for malformed templates
4. **Performance Tests**: Ensure conversion doesn't significantly impact performance
5. **Edge Case Tests**: Handle complex nested structures and unusual syntax

### Integration Test Scenarios
```rust
#[tokio::test]
async fn test_complex_conditional_rendering() {
    let template = r#"
{% if user.role == "admin" %}
  Access Level: Administrator
  {% if system.debug %}
    Debug Mode: Enabled
  {% else %}
    Debug Mode: Disabled
  {% endif %}
{% else %}
  Access Level: User
{% endif %}
"#;
    
    let variables = json!({
        "user": {"role": "admin"},
        "system": {"debug": true}
    });
    
    let processor = AdvancedTemplateProcessor::new();
    let result = processor.render_template(template, &variables).unwrap();
    
    assert!(result.contains("Access Level: Administrator"));
    assert!(result.contains("Debug Mode: Enabled"));
}

#[tokio::test]
async fn test_complex_loop_with_objects() {
    let template = r#"
{% for server in servers %}
  Server: {{ server.name }}
  Host: {{ server.host }}
  {% if server.ssl_enabled %}
    SSL: Enabled
  {% endif %}
{% endfor %}
"#;
    
    let variables = json!({
        "servers": [
            {"name": "web1", "host": "192.168.1.10", "ssl_enabled": true},
            {"name": "web2", "host": "192.168.1.11", "ssl_enabled": false}
        ]
    });
    
    let processor = AdvancedTemplateProcessor::new();
    let result = processor.render_template(template, &variables).unwrap();
    
    assert!(result.contains("Server: web1"));
    assert!(result.contains("SSL: Enabled"));
    assert!(!result.contains("SSL: Enabled") || result.matches("SSL: Enabled").count() == 1);
}
```

## Edge Cases & Error Handling

### Template Syntax Edge Cases
- **Nested loops with same variable names**: Handle variable scoping correctly
- **Complex conditionals with multiple operators**: Support `and`, `or`, `not` operators  
- **Whitespace-sensitive templates**: Preserve indentation and formatting
- **Mixed Jinja2 and Handlebars syntax**: Graceful handling or clear error messages
- **Empty arrays and null values**: Proper handling in loops and conditionals

### Error Recovery Strategies
```rust
impl TemplateError {
    pub fn with_context(self, template_line: usize, content: &str) -> Self {
        match self {
            TemplateError::ConversionFailed { message } => {
                TemplateError::ConversionFailed {
                    message: format!("{} at line {}: {}", message, template_line, content)
                }
            }
            _ => self,
        }
    }
}

impl AdvancedTemplateProcessor {
    fn handle_conversion_error(&self, error: ParseError, template: &str) -> TemplateError {
        // Provide detailed error context including line numbers and suggestions
        let line_number = self.find_error_line(&error, template);
        let suggestion = self.suggest_fix(&error);
        
        TemplateError::ConversionFailed {
            message: format!("Line {}: {} (suggestion: {})", line_number, error, suggestion)
        }
    }
}
```

## Dependencies

### External Dependencies
- **regex = "1.10"** (already available) - Enhanced pattern matching for complex syntax
- **handlebars = "6.3"** (already available) - Core template rendering engine
- **thiserror = "2.0"** (already available) - Error handling enhancements

### Internal Dependencies
- Enhanced `crate::modules::files::template` - Core template module integration
- `crate::modules::error` - Error type integration for template failures
- Test fixtures in `tests::modules::files::helpers::fixtures` - Complex template test cases

### Optional Performance Dependencies
- **once_cell = "1.19"** - Cache compiled regex patterns for better performance
- **rayon = "1.10"** - Parallel template processing for large templates (if needed)

## Configuration

### Template Processing Configuration
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedTemplateConfig {
    pub enable_jinja2_conversion: bool,           // Default: true
    pub strict_syntax_checking: bool,             // Default: false  
    pub max_template_size: usize,                 // Default: 10MB
    pub max_nesting_depth: usize,                 // Default: 50
    pub cache_converted_templates: bool,          // Default: true
    pub whitespace_control: WhitespaceControl,    // Default: Preserve
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WhitespaceControl {
    Preserve,      // Keep all whitespace as-is
    Trim,          // Remove leading/trailing whitespace
    Minimize,      // Reduce multiple whitespace to single spaces
}
```

### Runtime Configuration
```rust
impl TemplateModule {
    pub fn with_config(config: AdvancedTemplateConfig) -> Self {
        let processor = AdvancedTemplateProcessor::with_config(config);
        Self { processor }
    }
}
```

## Documentation

### Enhanced Module Documentation
```rust
/// Advanced template processing module with comprehensive Jinja2 compatibility
/// 
/// This module provides sophisticated template rendering capabilities including:
/// - Full Jinja2 control structure support (if/else, for loops)
/// - Complex variable access with dot notation
/// - Nested template structures with proper scoping
/// - Advanced error handling with detailed feedback
/// 
/// # Supported Jinja2 Features
/// 
/// ## Conditionals
/// ```jinja2
/// {% if user.admin %}
///   Admin content
/// {% else %}
///   User content  
/// {% endif %}
/// ```
/// 
/// ## Loops
/// ```jinja2
/// {% for item in items %}
///   {{ item.name }}: {{ item.value }}
/// {% endfor %}
/// ```
/// 
/// ## Complex Variables
/// ```jinja2
/// {{ config.database.host | default('localhost') }}
/// {{ environment == 'production' }}
/// ```
/// 
/// # Examples
/// 
/// ```rust
/// let processor = AdvancedTemplateProcessor::new();
/// let template = "{% for user in users %}Hello {{ user.name }}!{% endfor %}";
/// let variables = json!({"users": [{"name": "Alice"}, {"name": "Bob"}]});
/// let result = processor.render_template(template, &variables)?;
/// assert_eq!(result, "Hello Alice!Hello Bob!");
/// ```
pub struct AdvancedTemplateProcessor;
```

### Migration Guide
```rust
/// Migration from basic template processing to advanced Jinja2 compatibility
/// 
/// # Backward Compatibility
/// All existing templates continue to work unchanged. New features are opt-in.
/// 
/// # New Features Available
/// - Control structures: if/else, for loops
/// - Complex variable access: object.property syntax
/// - Advanced conditionals: comparison operators
/// 
/// # Performance Considerations
/// - Template conversion adds ~10-20% processing overhead
/// - Compiled templates are cached to minimize repeated conversion cost
/// - Large templates (>1MB) may see increased memory usage during conversion
/// 
/// # Breaking Changes
/// None. All changes are additive and backward compatible.
```

## Implementation Priority

### Phase 1: Core Jinja2 Conversion (Week 1)
1. Implement basic conditional conversion (`if`/`else`/`endif`)
2. Add simple loop conversion (`for`/`endfor`)
3. Enhance variable access for dot notation
4. Create advanced template processor structure

### Phase 2: Complex Features (Week 2)  
5. Add nested structure support with proper scoping
6. Implement comparison operators and complex conditionals
7. Add advanced error handling with detailed feedback
8. Create comprehensive unit tests for all conversion types

### Phase 3: Integration & Polish (Week 3)
9. Integrate with existing template module infrastructure
10. Add performance optimizations and caching
11. Implement edge case handling and validation
12. Complete integration tests with complex scenarios

### Phase 4: Validation & Performance (Week 4)
13. Achieve 100% test pass rate for all template tests
14. Performance benchmark against baseline template processing
15. Documentation and migration guide completion
16. Final validation with real-world template examples

This specification ensures complete Jinja2 compatibility while maintaining the robust Handlebars-based architecture, resolving the remaining template test failures and providing a comprehensive template processing solution.