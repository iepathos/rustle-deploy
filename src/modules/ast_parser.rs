use crate::modules::error::CompileError;
use syn::{parse_str, File, Item, ItemFn};

/// AST-based code parser for module analysis
pub struct AstParser;

impl AstParser {
    pub fn new() -> Self {
        Self
    }

    /// Extract the execute function from module source code
    pub fn extract_execute_function(&self, source: &str) -> Result<Option<String>, CompileError> {
        let ast: File = parse_str(source).map_err(|e| CompileError::SyntaxError {
            message: format!("Failed to parse Rust code: {e}"),
        })?;

        for item in ast.items {
            if let Item::Fn(func) = item {
                if func.sig.ident == "execute" {
                    return Ok(Some(self.function_to_string(&func)?));
                }
            }
        }

        Ok(None)
    }

    /// Find all functions in the source code
    pub fn find_all_functions(&self, source: &str) -> Result<Vec<ItemFn>, CompileError> {
        let ast: File = parse_str(source).map_err(|e| CompileError::SyntaxError {
            message: format!("Failed to parse Rust code: {e}"),
        })?;

        let mut functions = Vec::new();
        for item in ast.items {
            if let Item::Fn(func) = item {
                functions.push(func);
            }
        }

        Ok(functions)
    }

    /// Convert a function AST back to string
    fn function_to_string(&self, func: &ItemFn) -> Result<String, CompileError> {
        use quote::ToTokens;

        let tokens = func.to_token_stream();
        Ok(tokens.to_string())
    }

    /// Transform module source by removing main and test functions
    pub fn prepare_module_source(&self, source: &str) -> Result<String, CompileError> {
        let mut ast: File = parse_str(source).map_err(|e| CompileError::SyntaxError {
            message: format!("Failed to parse Rust code: {e}"),
        })?;

        // Filter out main functions and test modules
        ast.items.retain(|item| match item {
            Item::Fn(func) => func.sig.ident != "main",
            Item::Mod(module) => module.ident != "tests",
            _ => true,
        });

        // Convert back to string
        use quote::ToTokens;
        let tokens = ast.to_token_stream();
        Ok(tokens.to_string())
    }

    /// Validate that a function has the expected signature for a module execute function
    pub fn validate_execute_signature(&self, func: &ItemFn) -> Result<(), CompileError> {
        // Check function name
        if func.sig.ident != "execute" {
            return Err(CompileError::ValidationFailed {
                reason: "Function must be named 'execute'".to_string(),
            });
        }

        // Check that it's async
        if func.sig.asyncness.is_none() {
            return Err(CompileError::ValidationFailed {
                reason: "Execute function must be async".to_string(),
            });
        }

        // Check return type (should return Result)
        if let syn::ReturnType::Default = func.sig.output {
            return Err(CompileError::ValidationFailed {
                reason: "Execute function must return a Result".to_string(),
            });
        }

        Ok(())
    }
}

impl Default for AstParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_execute_function() {
        let source = r#"
            use std::fs;
            
            async fn execute(args: &Args) -> Result<Output> {
                // Function body
                Ok(Output::default())
            }
            
            fn helper() {}
        "#;

        let parser = AstParser::new();
        let result = parser.extract_execute_function(source).unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("async fn execute"));
    }

    #[test]
    fn test_prepare_module_source() {
        let source = r#"
            fn main() {
                println!("Main function");
            }
            
            fn execute() {}
            
            mod tests {
                #[test]
                fn test_something() {}
            }
        "#;

        let parser = AstParser::new();
        let result = parser.prepare_module_source(source).unwrap();
        assert!(!result.contains("fn main"));
        assert!(!result.contains("mod tests"));
        assert!(result.contains("fn execute"));
    }

    #[test]
    fn test_error_on_invalid_syntax() {
        let invalid_source = r#"
            fn broken { // Missing parentheses
                println!("Invalid");
            }
        "#;

        let parser = AstParser::new();
        let result = parser.extract_execute_function(invalid_source);
        assert!(result.is_err());
        match result {
            Err(CompileError::SyntaxError { message }) => {
                assert!(message.contains("Failed to parse Rust code"));
            }
            _ => panic!("Expected SyntaxError"),
        }
    }

    #[test]
    fn test_no_execute_function() {
        let source = r#"
            fn other_function() {
                println!("Not execute");
            }
        "#;

        let parser = AstParser::new();
        let result = parser.extract_execute_function(source).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_execute_signature() {
        let valid_source = r#"
            async fn execute(args: &Args) -> Result<Output> {
                Ok(Output::default())
            }
        "#;

        let parser = AstParser::new();
        let ast: File = parse_str(valid_source).unwrap();
        if let Some(Item::Fn(func)) = ast.items.into_iter().next() {
            assert!(parser.validate_execute_signature(&func).is_ok());
        }

        let invalid_source = r#"
            fn execute(args: &Args) -> Result<Output> {
                Ok(Output::default())
            }
        "#;

        let ast: File = parse_str(invalid_source).unwrap();
        if let Some(Item::Fn(func)) = ast.items.into_iter().next() {
            let result = parser.validate_execute_signature(&func);
            assert!(result.is_err());
            match result {
                Err(CompileError::ValidationFailed { reason }) => {
                    assert!(reason.contains("must be async"));
                }
                _ => panic!("Expected ValidationFailed"),
            }
        }
    }
}
