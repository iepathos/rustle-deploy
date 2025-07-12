//! Advanced template processor with comprehensive Jinja2 compatibility

use handlebars::Handlebars;
use serde_json::Value;
use thiserror::Error;

use super::handlebars_helpers::{
    default_helper, equality_helper, greater_than_helper, less_than_helper, not_equal_helper,
    quote_helper,
};
use super::jinja_parser::{Jinja2Parser, ParseError};

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("Template rendering failed: {message}")]
    RenderingFailed { message: String },

    #[error("Template conversion failed: {message}")]
    ConversionFailed { message: String },

    #[error("Unbalanced template blocks: {block_type}")]
    UnbalancedBlocks { block_type: String },

    #[error("Template validation failed: {message}")]
    ValidationFailed { message: String },
}

impl From<handlebars::RenderError> for TemplateError {
    fn from(error: handlebars::RenderError) -> Self {
        TemplateError::RenderingFailed {
            message: error.to_string(),
        }
    }
}

impl From<ParseError> for TemplateError {
    fn from(error: ParseError) -> Self {
        TemplateError::ConversionFailed {
            message: error.to_string(),
        }
    }
}

/// Advanced template processor with comprehensive Jinja2 compatibility
pub struct AdvancedTemplateProcessor {
    handlebars: Handlebars<'static>,
    jinja_parser: Jinja2Parser,
}

impl AdvancedTemplateProcessor {
    pub fn new() -> Result<Self, TemplateError> {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(false);

        // Register advanced helpers
        handlebars.register_helper("default", Box::new(default_helper));
        handlebars.register_helper("quote", Box::new(quote_helper));
        handlebars.register_helper("eq", Box::new(equality_helper));
        handlebars.register_helper("ne", Box::new(not_equal_helper));
        handlebars.register_helper("gt", Box::new(greater_than_helper));
        handlebars.register_helper("lt", Box::new(less_than_helper));

        let jinja_parser = Jinja2Parser::new().map_err(|e| TemplateError::ConversionFailed {
            message: format!("Failed to initialize Jinja2 parser: {e}"),
        })?;

        Ok(Self {
            handlebars,
            jinja_parser,
        })
    }

    pub fn render_template(
        &self,
        template_content: &str,
        variables: &Value,
    ) -> Result<String, TemplateError> {
        // Validate template syntax
        self.validate_template_syntax(template_content)?;

        // Convert Jinja2 template to Handlebars
        let conversion_result = self.jinja_parser.convert_to_handlebars(template_content)?;

        // Render the converted template
        let rendered = self
            .handlebars
            .render_template(&conversion_result.handlebars_template, variables)?;

        Ok(rendered)
    }

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
        // Use the same logic as in Jinja2Parser for consistency
        let block_pattern = regex::Regex::new(r#"\{\%\s*(\w+)[^%]*\%\}"#).map_err(|e| {
            TemplateError::ValidationFailed {
                message: format!("Regex error: {e}"),
            }
        })?;

        let mut if_stack = 0;
        let mut for_stack = 0;

        for caps in block_pattern.captures_iter(template) {
            let block_type = &caps[1];

            match block_type {
                "if" => if_stack += 1,
                "elif" => {
                    // elif is valid only inside if blocks
                    if if_stack == 0 {
                        return Err(TemplateError::UnbalancedBlocks {
                            block_type: "elif without if".to_string(),
                        });
                    }
                }
                "else" => {
                    // else can be in if or for blocks, no special validation needed
                }
                "endif" => {
                    if_stack -= 1;
                    if if_stack < 0 {
                        return Err(TemplateError::UnbalancedBlocks {
                            block_type: "unmatched endif".to_string(),
                        });
                    }
                }
                "for" => for_stack += 1,
                "endfor" => {
                    for_stack -= 1;
                    if for_stack < 0 {
                        return Err(TemplateError::UnbalancedBlocks {
                            block_type: "unmatched endfor".to_string(),
                        });
                    }
                }
                _ => {
                    // Ignore other block types for now
                }
            }
        }

        if if_stack != 0 {
            return Err(TemplateError::UnbalancedBlocks {
                block_type: format!("if (missing {if_stack} endif)"),
            });
        }

        if for_stack != 0 {
            return Err(TemplateError::UnbalancedBlocks {
                block_type: format!("for (missing {for_stack} endfor)"),
            });
        }

        Ok(())
    }

    fn validate_variable_syntax(&self, template: &str) -> Result<(), TemplateError> {
        // Basic validation for malformed variable references
        let lines: Vec<&str> = template.lines().collect();
        for (line_num, line) in lines.iter().enumerate() {
            // Check for unclosed variable references
            let open_count = line.matches("{{").count();
            let close_count = line.matches("}}").count();
            if open_count != close_count {
                return Err(TemplateError::ValidationFailed {
                    message: format!(
                        "Unclosed variable reference at line {}: {}",
                        line_num + 1,
                        line.trim()
                    ),
                });
            }

            // Check for unclosed control blocks
            let block_open_count = line.matches("{%").count();
            let block_close_count = line.matches("%}").count();
            if block_open_count != block_close_count {
                return Err(TemplateError::ValidationFailed {
                    message: format!(
                        "Unclosed control block at line {}: {}",
                        line_num + 1,
                        line.trim()
                    ),
                });
            }
        }

        Ok(())
    }

    fn check_unsupported_features(&self, template: &str) -> Result<(), TemplateError> {
        // Check for advanced Jinja2 features that are not yet supported
        let unsupported_patterns = vec![
            ("{% set ", "variable assignment"),
            ("{% include ", "template inclusion"),
            ("{% extends ", "template inheritance"),
            ("{% block ", "template blocks"),
            ("{% macro ", "macros"),
            ("{% raw ", "raw blocks"),
            ("{% filter ", "filter blocks"),
        ];

        for (pattern, feature_name) in unsupported_patterns {
            if template.contains(pattern) {
                return Err(TemplateError::ValidationFailed {
                    message: format!("Unsupported Jinja2 feature: {feature_name}"),
                });
            }
        }

        Ok(())
    }

    pub fn get_conversion_info(
        &self,
        template_content: &str,
    ) -> Result<super::jinja_parser::ConversionResult, TemplateError> {
        self.jinja_parser
            .convert_to_handlebars(template_content)
            .map_err(Into::into)
    }
}

impl Default for AdvancedTemplateProcessor {
    fn default() -> Self {
        Self::new().expect("Failed to create AdvancedTemplateProcessor")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_simple_variable_substitution() {
        let processor = AdvancedTemplateProcessor::new().unwrap();
        let template = "Hello {{name}}!";
        let variables = json!({"name": "World"});

        let result = processor.render_template(template, &variables).unwrap();
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_default_filter() {
        let processor = AdvancedTemplateProcessor::new().unwrap();
        let template = "Host: {{host | default('localhost')}}";
        let variables = json!({});

        let result = processor.render_template(template, &variables).unwrap();
        assert_eq!(result, "Host: localhost");
    }

    #[test]
    fn test_conditional_rendering() {
        let processor = AdvancedTemplateProcessor::new().unwrap();
        let template = "{% if enable_ssl %}SSL: enabled{% else %}SSL: disabled{% endif %}";

        let variables_ssl = json!({"enable_ssl": true});
        let result = processor.render_template(template, &variables_ssl).unwrap();
        assert!(result.contains("SSL: enabled"));

        let variables_no_ssl = json!({"enable_ssl": false});
        let result = processor
            .render_template(template, &variables_no_ssl)
            .unwrap();
        assert!(result.contains("SSL: disabled"));
    }

    #[test]
    fn test_loop_rendering() {
        let processor = AdvancedTemplateProcessor::new().unwrap();
        let template = "{% for item in items %}{{item.name}} {% endfor %}";
        let variables = json!({
            "items": [
                {"name": "Alice"},
                {"name": "Bob"}
            ]
        });

        let result = processor.render_template(template, &variables).unwrap();
        assert!(result.contains("Alice"));
        assert!(result.contains("Bob"));
    }

    #[test]
    fn test_complex_template() {
        let processor = AdvancedTemplateProcessor::new().unwrap();
        let template = r#"
# Configuration for {{service_name}}
{% if enable_ssl %}
ssl_enabled = true
ssl_cert = "{{ssl_cert_path}}"
{% else %}
ssl_enabled = false
{% endif %}

{% for server in servers %}
[[servers.list]]
name = "{{server.name}}"
host = "{{server.host}}"
{% endfor %}
"#;

        let variables = json!({
            "service_name": "web_service",
            "enable_ssl": true,
            "ssl_cert_path": "/etc/ssl/cert.pem",
            "servers": [
                {"name": "web1", "host": "192.168.1.10"},
                {"name": "web2", "host": "192.168.1.11"}
            ]
        });

        let result = processor.render_template(template, &variables).unwrap();
        assert!(result.contains("web_service"));
        assert!(result.contains("ssl_enabled = true"));
        assert!(result.contains("/etc/ssl/cert.pem"));
        assert!(result.contains("web1"));
        assert!(result.contains("web2"));
    }

    #[test]
    fn test_unbalanced_blocks_error() {
        let processor = AdvancedTemplateProcessor::new().unwrap();
        let template = "{% if condition %}content"; // Missing endif
        let variables = json!({});

        let result = processor.render_template(template, &variables);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unbalanced"));
    }

    #[test]
    fn test_unsupported_features_error() {
        let processor = AdvancedTemplateProcessor::new().unwrap();
        let template = "{% set var = 'value' %}{{var}}"; // set is not supported
        let variables = json!({});

        let result = processor.render_template(template, &variables);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }
}
