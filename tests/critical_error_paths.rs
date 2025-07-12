//! Tests for critical error paths that were fixed

#[cfg(test)]
mod inventory_error_tests {
    use rustle_deploy::inventory::error::VariableError;
    use rustle_deploy::inventory::variables::VariableResolver;
    use rustle_deploy::types::inventory::InventoryGroup;
    use std::collections::HashMap;

    #[test]
    fn test_circular_group_dependency_detection() {
        let resolver = VariableResolver::new();
        let mut groups = HashMap::new();

        // Create circular dependency: A -> B -> C -> A
        groups.insert(
            "group_a".to_string(),
            InventoryGroup {
                name: "group_a".to_string(),
                hosts: vec![],
                children: vec![],
                parent_groups: vec!["group_c".to_string()],
                variables: HashMap::new(),
            },
        );
        groups.insert(
            "group_b".to_string(),
            InventoryGroup {
                name: "group_b".to_string(),
                hosts: vec![],
                children: vec![],
                parent_groups: vec!["group_a".to_string()],
                variables: HashMap::new(),
            },
        );
        groups.insert(
            "group_c".to_string(),
            InventoryGroup {
                name: "group_c".to_string(),
                hosts: vec![],
                children: vec![],
                parent_groups: vec!["group_b".to_string()],
                variables: HashMap::new(),
            },
        );

        let result = resolver.validate_no_circular_dependencies(&groups);
        assert!(result.is_err());
        match result.unwrap_err() {
            VariableError::CircularDependency { cycle } => {
                assert!(!cycle.is_empty());
                // The cycle should contain all three groups
                assert!(cycle.len() >= 3);
            }
            _ => panic!("Expected CircularDependency error"),
        }
    }
}

#[cfg(test)]
mod ast_parser_error_tests {
    use rustle_deploy::modules::ast_parser::AstParser;

    #[test]
    fn test_parse_invalid_rust_syntax() {
        let parser = AstParser::new();
        let invalid_code = r#"
            fn broken syntax here {
                this is not valid rust
            }
        "#;

        let result = parser.extract_execute_function(invalid_code);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Failed to parse Rust code"));
    }

    #[test]
    fn test_extract_function_from_empty_module() {
        let parser = AstParser::new();
        let empty_code = "";

        let result = parser.extract_execute_function(empty_code);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_extract_function_without_execute() {
        let parser = AstParser::new();
        let code = r#"
            fn other_function() {
                println!("Not execute");
            }
            
            fn another_function() {
                println!("Also not execute");
            }
        "#;

        let result = parser.extract_execute_function(code);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_prepare_source_removes_main() {
        let parser = AstParser::new();
        let code = r#"
            fn main() {
                println!("Should be removed");
            }
            
            fn keep_this() {
                println!("Should remain");
            }
        "#;

        let result = parser.prepare_module_source(code);
        assert!(result.is_ok());
        let prepared = result.unwrap();
        assert!(!prepared.contains("fn main"));
        assert!(prepared.contains("fn keep_this"));
    }
}

#[cfg(test)]
mod resolve_error_tests {
    use rustle_deploy::execution::plan::ModuleSource;
    use rustle_deploy::modules::resolver::FileSystemResolver;
    use rustle_deploy::modules::ModuleSourceResolver;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_resolve_nonexistent_module() {
        let resolver = FileSystemResolver::new(vec![PathBuf::from("/tmp/nonexistent")]);
        let source = ModuleSource::File {
            path: "definitely_not_a_real_module.rs".to_string(),
        };

        let result = resolver.resolve(&source).await;
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod deployment_path_error_tests {
    use std::path::Path;

    #[test]
    fn test_path_parent_handling() {
        // Test root path has no parent
        let root_path = Path::new("/");
        assert!(root_path.parent().is_none() || root_path.parent() == Some(Path::new("/")));

        // Test normal path has parent
        let normal_path = Path::new("/usr/local/bin/app");
        assert!(normal_path.parent().is_some());
        assert_eq!(normal_path.parent().unwrap(), Path::new("/usr/local/bin"));

        // Test relative path
        let relative_path = Path::new("file.txt");
        assert!(relative_path.parent() == Some(Path::new("")));
    }
}

#[cfg(test)]
mod template_cache_error_tests {
    use rustle_deploy::template::cache::TemplateCache;

    #[test]
    fn test_cache_operations() {
        // Test that cache operations work correctly
        let cache = TemplateCache::new(true);

        // Test getting from empty cache
        let result = cache.get("nonexistent");
        assert!(result.is_none());

        // Test with disabled cache
        let disabled_cache = TemplateCache::new(false);
        let result = disabled_cache.get("anything");
        assert!(result.is_none());
    }
}
