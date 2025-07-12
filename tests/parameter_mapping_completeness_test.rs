// Include the parameter mapping modules for testing
#[path = "../src/templates/modules/parameter_mapping/mod.rs"]
mod parameter_mapping;

#[test]
fn test_all_supported_modules_have_parameter_handlers() {
    // We'll test with a known list of modules that should be supported
    let supported_modules = vec![
        "file".to_string(),
        "copy".to_string(),
        "command".to_string(),
        "shell".to_string(),
        "debug".to_string(),
        "package".to_string(),
        "service".to_string(),
    ];

    // Get all modules that have parameter handlers
    let mapper = parameter_mapping::ParameterMapper::new();

    // Test each supported module to see if it has a parameter handler
    let mut missing_handlers = Vec::new();
    let mut working_handlers = Vec::new();

    for module_name in &supported_modules {
        let test_params = std::collections::HashMap::new();
        match mapper.map_for_module(module_name, test_params) {
            Ok(_) | Err(parameter_mapping::ParameterError::MissingRequired { .. }) => {
                // Handler exists (either succeeded or failed validation, which means it was found)
                working_handlers.push(module_name.clone());
            }
            Err(parameter_mapping::ParameterError::UnknownParameter { .. }) => {
                // No handler for this module
                missing_handlers.push(module_name.clone());
            }
            Err(_) => {
                // Other error - handler exists but might have other issues
                working_handlers.push(module_name.clone());
            }
        }
    }

    // Report findings
    println!(
        "Modules with working parameter handlers: {:?}",
        working_handlers
    );
    println!("Modules missing parameter handlers: {:?}", missing_handlers);

    // Fail the test if any supported modules are missing parameter handlers
    assert!(
        missing_handlers.is_empty(),
        "The following supported modules are missing parameter handlers: {:?}. \
         All modules in the binary registry should have corresponding parameter handlers \
         to ensure proper Ansible compatibility.",
        missing_handlers
    );
}

#[test]
fn test_essential_modules_are_registered() {
    let mapper = parameter_mapping::ParameterMapper::new();

    // These are the essential modules that should always have parameter handlers
    let essential_modules = [
        "file", "copy", "command", "shell", "debug", "package", "service",
    ];

    let mut missing_essential = Vec::new();

    for module_name in &essential_modules {
        let test_params = std::collections::HashMap::new();
        match mapper.map_for_module(module_name, test_params) {
            Err(parameter_mapping::ParameterError::UnknownParameter { .. }) => {
                missing_essential.push(*module_name);
            }
            _ => {
                // Handler exists (any other result means the handler was found)
            }
        }
    }

    assert!(
        missing_essential.is_empty(),
        "Essential modules missing parameter handlers: {:?}",
        missing_essential
    );
}

#[test]
fn test_parameter_handler_consistency() {
    let mapper = parameter_mapping::ParameterMapper::new();

    // Test that known modules behave consistently
    let test_cases = vec![
        (
            "file",
            vec!["path"],
            vec!["mode", "state", "owner", "group"],
        ),
        ("copy", vec!["src", "dest"], vec!["mode", "backup", "force"]),
        ("command", vec![], vec!["cmd", "_raw_params"]), // command is flexible
        ("debug", vec![], vec!["msg", "var"]),           // debug needs msg OR var
        ("package", vec!["name"], vec!["state"]),
        ("service", vec!["name"], vec!["state"]),
    ];

    for (module_name, required_params, _optional_params) in test_cases {
        // Test with empty params - should fail for modules with required params
        let empty_result = mapper.map_for_module(module_name, std::collections::HashMap::new());

        if !required_params.is_empty() {
            assert!(
                empty_result.is_err(),
                "Module '{}' should fail with empty parameters since it has required params: {:?}",
                module_name,
                required_params
            );
        }

        // Test with all required params
        let mut full_params = std::collections::HashMap::new();
        for param in &required_params {
            full_params.insert(
                param.to_string(),
                serde_json::Value::String("test_value".to_string()),
            );
        }

        // Special cases for modules that need specific parameters
        if module_name == "debug" {
            // Debug module needs msg OR var, not both
            full_params.clear();
            full_params.insert(
                "msg".to_string(),
                serde_json::Value::String("test message".to_string()),
            );
        } else if module_name == "command" {
            // Command module needs cmd parameter
            full_params.insert(
                "cmd".to_string(),
                serde_json::Value::String("/bin/true".to_string()),
            );
        }

        let full_result = mapper.map_for_module(module_name, full_params);
        assert!(
            full_result.is_ok()
                || matches!(
                    full_result,
                    Err(parameter_mapping::ParameterError::ConflictingParameters { .. })
                ),
            "Module '{}' should succeed with required parameters, but got: {:?}",
            module_name,
            full_result
        );
    }
}

#[test]
fn test_unknown_module_handling() {
    let mapper = parameter_mapping::ParameterMapper::new();

    let result = mapper.map_for_module("nonexistent_module", std::collections::HashMap::new());

    assert!(result.is_err());
    match result.unwrap_err() {
        parameter_mapping::ParameterError::UnknownParameter { param } => {
            assert!(param.contains("nonexistent_module"));
        }
        other => panic!("Expected UnknownParameter error, got: {:?}", other),
    }
}

#[test]
fn test_parameter_mapping_robustness() {
    let mapper = parameter_mapping::ParameterMapper::new();

    // Test with various edge cases
    let long_name = "a".repeat(1000);
    let edge_cases = vec![
        // Empty string module name
        ("", std::collections::HashMap::new()),
        // Module name with special characters
        ("file@test", std::collections::HashMap::new()),
        // Very long module name
        (long_name.as_str(), std::collections::HashMap::new()),
    ];

    for (module_name, params) in edge_cases {
        let result = mapper.map_for_module(module_name, params);
        // Should either work or fail gracefully with UnknownParameter
        match result {
            Ok(_) => {}                                                           // Fine if it works
            Err(parameter_mapping::ParameterError::UnknownParameter { .. }) => {} // Expected for unknown modules
            Err(other) => {
                // Other errors are OK too (like validation errors) as long as we don't panic
                println!("Edge case '{}' failed with: {:?}", module_name, other);
            }
        }
    }
}
