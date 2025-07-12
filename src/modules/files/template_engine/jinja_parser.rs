//! Jinja2 to Handlebars template conversion parser

use regex::Regex;
use thiserror::Error;

#[derive(Debug, Error)]
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

impl From<regex::Error> for ParseError {
    fn from(error: regex::Error) -> Self {
        ParseError::RegexError {
            error: error.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConversionResult {
    pub handlebars_template: String,
    pub required_helpers: Vec<String>,
    pub variable_paths: Vec<String>,
    pub warnings: Vec<String>,
}

pub struct Jinja2Parser {
    variable_regex: Regex,
    default_filter_regex: Regex,
    default_filter_numeric_regex: Regex,
}

impl Jinja2Parser {
    pub fn new() -> Result<Self, ParseError> {
        Ok(Self {
            variable_regex: Regex::new(r#"\{\{\s*(\w+)\.(\w+)\s*\}\}"#)?,
            default_filter_regex: Regex::new(r#"\{\{\s*(\w+)\s*\|\s*default\('([^']*)'\)\s*\}\}"#)?,
            default_filter_numeric_regex: Regex::new(
                r#"\{\{\s*(\w+)\s*\|\s*default\(([^)]*)\)\s*\}\}"#,
            )?,
        })
    }

    pub fn convert_to_handlebars(
        &self,
        jinja_template: &str,
    ) -> Result<ConversionResult, ParseError> {
        let mut template = jinja_template.to_string();
        let mut required_helpers = Vec::new();
        let mut variable_paths = Vec::new();
        let warnings = Vec::new();

        // Validate template syntax first
        self.validate_template_syntax(&template)?;

        // Convert default filters first (before other conversions that might interfere)
        template = self.convert_default_filters(&template, &mut required_helpers)?;

        // Convert conditionals
        template = self.convert_conditionals(&template)?;

        // Convert loops
        template = self.convert_loops(&template, &mut variable_paths)?;

        // Convert variable access patterns
        template = self.convert_variables(&template, &mut variable_paths)?;

        Ok(ConversionResult {
            handlebars_template: template,
            required_helpers,
            variable_paths,
            warnings,
        })
    }

    fn validate_template_syntax(&self, template: &str) -> Result<(), ParseError> {
        // Check for balanced control structures
        self.check_balanced_blocks(template)?;
        Ok(())
    }

    fn check_balanced_blocks(&self, template: &str) -> Result<(), ParseError> {
        let mut if_stack = 0;
        let mut for_stack = 0;

        // Use regex to find all blocks in the template
        let block_pattern =
            Regex::new(r#"\{\%\s*(\w+)[^%]*\%\}"#).map_err(|e| ParseError::RegexError {
                error: e.to_string(),
            })?;

        for caps in block_pattern.captures_iter(template) {
            let block_type = &caps[1];

            match block_type {
                "if" => if_stack += 1,
                "elif" => {
                    // elif is valid only inside if blocks
                    if if_stack == 0 {
                        return Err(ParseError::InvalidConditional {
                            syntax: caps[0].to_string(),
                        });
                    }
                }
                "else" => {
                    // else can be in if or for blocks, no special validation needed
                }
                "endif" => {
                    if_stack -= 1;
                    if if_stack < 0 {
                        return Err(ParseError::UnclosedBlock {
                            block_type: "if".to_string(),
                        });
                    }
                }
                "for" => for_stack += 1,
                "endfor" => {
                    for_stack -= 1;
                    if for_stack < 0 {
                        return Err(ParseError::UnclosedBlock {
                            block_type: "for".to_string(),
                        });
                    }
                }
                _ => {
                    // Ignore other block types for now
                }
            }
        }

        if if_stack != 0 {
            return Err(ParseError::UnclosedBlock {
                block_type: "if".to_string(),
            });
        }

        if for_stack != 0 {
            return Err(ParseError::UnclosedBlock {
                block_type: "for".to_string(),
            });
        }

        Ok(())
    }

    fn convert_default_filters(
        &self,
        template: &str,
        required_helpers: &mut Vec<String>,
    ) -> Result<String, ParseError> {
        let mut result = template.to_string();

        // Add 'default' to required helpers
        if !required_helpers.contains(&"default".to_string()) {
            required_helpers.push("default".to_string());
        }

        // Convert {{ var | default('value') }} to {{default var 'value'}}
        result = self
            .default_filter_regex
            .replace_all(&result, "{{default $1 '$2'}}")
            .to_string();

        // Convert {{ var | default(value) }} to {{default var value}}
        result = self
            .default_filter_numeric_regex
            .replace_all(&result, "{{default $1 $2}}")
            .to_string();

        Ok(result)
    }

    fn convert_conditionals(&self, template: &str) -> Result<String, ParseError> {
        let mut result = template.to_string();

        // First, convert comparison operations in if statements
        let if_comparison_pattern =
            Regex::new(r#"\{\%\s*if\s+(\w+)\s*(==|!=)\s*"([^"]+)"\s*\%\}"#)?;
        result = if_comparison_pattern
            .replace_all(&result, |caps: &regex::Captures| {
                let var = &caps[1];
                let op = &caps[2];
                let value = &caps[3];

                match op {
                    "==" => format!("{{{{#if (eq {var} '{value}')}}}}"),
                    "!=" => format!("{{{{#if (ne {var} '{value}')}}}}"),
                    _ => caps[0].to_string(), // fallback
                }
            })
            .to_string();

        // Convert simple {% if condition %} to {{#if condition}}
        let if_pattern = Regex::new(r#"\{\%\s*if\s+([^%]+?)\s*\%\}"#)?;
        result = if_pattern.replace_all(&result, "{{#if $1}}").to_string();

        // Convert {% else %} to {{else}}
        let else_pattern = Regex::new(r#"\{\%\s*else\s*\%\}"#)?;
        result = else_pattern.replace_all(&result, "{{else}}").to_string();

        // Convert {% elif condition %} to {{else}}{{#if condition}}
        let elif_pattern = Regex::new(r#"\{\%\s*elif\s+([^%]+)\s*\%\}"#)?;
        result = elif_pattern
            .replace_all(&result, "{{else}}{{#if $1}}")
            .to_string();

        // Convert {% endif %} to {{/if}}
        let endif_pattern = Regex::new(r#"\{\%\s*endif\s*\%\}"#)?;
        result = endif_pattern.replace_all(&result, "{{/if}}").to_string();

        Ok(result)
    }

    fn convert_loops(
        &self,
        template: &str,
        variable_paths: &mut Vec<String>,
    ) -> Result<String, ParseError> {
        let mut result = template.to_string();

        // Convert {% for item in array %} to {{#each array}}
        let for_pattern = Regex::new(r#"\{\%\s*for\s+(\w+)\s+in\s+(\w+)\s*\%\}"#)?;

        // Capture loop variable names for later conversion
        for caps in for_pattern.captures_iter(&result) {
            let loop_var = caps
                .get(1)
                .ok_or_else(|| ParseError::RegexError {
                    error: "Failed to capture loop variable".to_string(),
                })?
                .as_str();
            let array_var = caps
                .get(2)
                .ok_or_else(|| ParseError::RegexError {
                    error: "Failed to capture array variable".to_string(),
                })?
                .as_str();
            variable_paths.push(format!("{array_var}[].{loop_var}"));
        }

        result = for_pattern.replace_all(&result, "{{#each $2}}").to_string();

        // Convert {% endfor %} to {{/each}}
        let endfor_pattern = Regex::new(r#"\{\%\s*endfor\s*\%\}"#)?;
        result = endfor_pattern.replace_all(&result, "{{/each}}").to_string();

        // Convert loop variable references within each blocks
        result = self.convert_loop_variables(&result)?;

        Ok(result)
    }

    fn convert_loop_variables(&self, template: &str) -> Result<String, ParseError> {
        let mut result = template.to_string();

        // Find each block boundaries and convert variables within them
        let each_blocks = self.find_each_blocks(&result)?;
        for block in each_blocks {
            let converted_block = self.convert_variables_in_block(&block)?;
            result = result.replace(&block.original, &converted_block);
        }

        Ok(result)
    }

    fn find_each_blocks(&self, template: &str) -> Result<Vec<EachBlock>, ParseError> {
        let mut blocks = Vec::new();
        let chars: Vec<char> = template.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if let Some(start_pos) = self.find_each_start(&chars, i) {
                if let Some(end_pos) = self.find_matching_each_end(&chars, start_pos) {
                    let original = chars[i..=end_pos].iter().collect::<String>();
                    blocks.push(EachBlock { original });
                    i = end_pos + 1;
                } else {
                    return Err(ParseError::UnclosedBlock {
                        block_type: "each".to_string(),
                    });
                }
            } else {
                i += 1;
            }
        }

        Ok(blocks)
    }

    fn find_each_start(&self, chars: &[char], start: usize) -> Option<usize> {
        let search_str = "{{#each";
        for i in start..chars.len() {
            if i + search_str.len() <= chars.len() {
                let slice: String = chars[i..i + search_str.len()].iter().collect();
                if slice == search_str {
                    return Some(i);
                }
            }
        }
        None
    }

    fn find_matching_each_end(&self, chars: &[char], start: usize) -> Option<usize> {
        let mut depth = 1;
        let mut i = start + "{{#each".len();

        while i < chars.len() && depth > 0 {
            if i + "{{#each".len() <= chars.len() {
                let slice: String = chars[i..i + "{{#each".len()].iter().collect();
                if slice == "{{#each" {
                    depth += 1;
                    i += "{{#each".len();
                    continue;
                }
            }

            if i + "{{/each}}".len() <= chars.len() {
                let slice: String = chars[i..i + "{{/each}}".len()].iter().collect();
                if slice == "{{/each}}" {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i + "{{/each}}".len() - 1);
                    }
                    i += "{{/each}}".len();
                    continue;
                }
            }

            i += 1;
        }

        None
    }

    fn convert_variables_in_block(&self, block: &EachBlock) -> Result<String, ParseError> {
        let mut result = block.original.clone();

        // Convert {{ item.property }} to {{ property }} within each blocks
        // This is a simplified approach - in real Jinja2, this would require more context awareness
        let item_property_pattern = Regex::new(r#"\{\{\s*\w+\.(\w+)\s*\}\}"#)?;
        result = item_property_pattern
            .replace_all(&result, "{{$1}}")
            .to_string();

        Ok(result)
    }

    fn convert_variables(
        &self,
        template: &str,
        variable_paths: &mut Vec<String>,
    ) -> Result<String, ParseError> {
        let mut result = template.to_string();

        // Convert dot notation for root level variables
        // {{ object.property }} stays as {{ object.property }} in Handlebars
        for caps in self.variable_regex.captures_iter(&result) {
            let object = caps
                .get(1)
                .ok_or_else(|| ParseError::RegexError {
                    error: "Failed to capture object".to_string(),
                })?
                .as_str();
            let property = caps
                .get(2)
                .ok_or_else(|| ParseError::RegexError {
                    error: "Failed to capture property".to_string(),
                })?
                .as_str();
            variable_paths.push(format!("{object}.{property}"));
        }

        // Handle comparison operations (basic implementation)
        let comparison_pattern = Regex::new(r#"\{\{\s*(\w+)\s*(==|!=)\s*"([^"]+)"\s*\}\}"#)?;
        result = comparison_pattern
            .replace_all(&result, "{{eq $1 '$3'}}")
            .to_string();

        Ok(result)
    }
}

#[derive(Debug)]
struct EachBlock {
    original: String,
}

impl Default for Jinja2Parser {
    fn default() -> Self {
        Self::new().expect("Failed to create Jinja2Parser")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_simple_conditionals() {
        let parser = Jinja2Parser::new().unwrap();
        let template = "{% if condition %}true{% else %}false{% endif %}";
        let result = parser.convert_to_handlebars(template).unwrap();
        assert_eq!(
            result.handlebars_template,
            "{{#if condition}}true{{else}}false{{/if}}"
        );
    }

    #[test]
    fn test_convert_simple_loops() {
        let parser = Jinja2Parser::new().unwrap();
        let template = "{% for item in items %}{{item}}{% endfor %}";
        let result = parser.convert_to_handlebars(template).unwrap();
        assert_eq!(
            result.handlebars_template,
            "{{#each items}}{{item}}{{/each}}"
        );
    }

    #[test]
    fn test_convert_default_filters() {
        let parser = Jinja2Parser::new().unwrap();
        let template = "{{name | default('unknown')}} {{port | default(8080)}}";
        let result = parser.convert_to_handlebars(template).unwrap();
        assert_eq!(
            result.handlebars_template,
            "{{default name 'unknown'}} {{default port 8080}}"
        );
        assert!(result.required_helpers.contains(&"default".to_string()));
    }

    #[test]
    fn test_validate_balanced_blocks() {
        let parser = Jinja2Parser::new().unwrap();

        // Valid template
        let valid = "{% if x %}{% for y in z %}{{y}}{% endfor %}{% endif %}";
        assert!(parser.validate_template_syntax(valid).is_ok());

        // Invalid template (unclosed if)
        let invalid = "{% if x %}content";
        assert!(parser.validate_template_syntax(invalid).is_err());
    }
}
