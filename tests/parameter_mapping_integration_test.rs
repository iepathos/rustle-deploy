use serde_json::Value;
use std::collections::HashMap;

// Include the parameter mapping modules for testing
#[path = "../src/templates/modules/parameter_mapping/mod.rs"]
mod parameter_mapping;

#[test]
fn test_command_raw_params_mapping() {
    let mapper = parameter_mapping::ParameterMapper::new();
    let mut params = HashMap::new();
    params.insert(
        "_raw_params".to_string(),
        Value::String("/bin/true".to_string()),
    );

    let mapped = mapper.map_for_module("command", params).unwrap();

    assert_eq!(mapped.get("cmd").unwrap().as_str().unwrap(), "/bin/true");
    assert!(!mapped.contains_key("_raw_params"));
}

#[test]
fn test_package_name_alias() {
    let mapper = parameter_mapping::ParameterMapper::new();
    let mut params = HashMap::new();
    params.insert("name".to_string(), Value::String("git".to_string()));
    params.insert("state".to_string(), Value::String("present".to_string()));

    let mapped = mapper.map_for_module("package", params).unwrap();

    assert_eq!(mapped.get("name").unwrap().as_str().unwrap(), "git");
    assert_eq!(mapped.get("state").unwrap().as_str().unwrap(), "present");
}

#[test]
fn test_debug_msg_parameter() {
    let mapper = parameter_mapping::ParameterMapper::new();
    let mut params = HashMap::new();
    params.insert("msg".to_string(), Value::String("hello world".to_string()));

    let mapped = mapper.map_for_module("debug", params).unwrap();

    assert_eq!(mapped.get("msg").unwrap().as_str().unwrap(), "hello world");
}

#[test]
fn test_missing_required_parameter() {
    let mapper = parameter_mapping::ParameterMapper::new();
    let params = HashMap::new();

    let result = mapper.map_for_module("command", params);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Missing required parameter"));
}

#[test]
fn test_example_plan_compatibility() {
    // Test the specific case from example_rustle_plan_output.json
    let mapper = parameter_mapping::ParameterMapper::new();

    // Task 2 from the example plan uses _raw_params
    let mut task_2_params = HashMap::new();
    task_2_params.insert(
        "_raw_params".to_string(),
        Value::String("/bin/true".to_string()),
    );

    let mapped = mapper.map_for_module("command", task_2_params).unwrap();
    assert_eq!(mapped.get("cmd").unwrap().as_str().unwrap(), "/bin/true");

    // Task 1 from the example plan uses package module
    let mut task_1_params = HashMap::new();
    task_1_params.insert("name".to_string(), Value::String("git".to_string()));
    task_1_params.insert("state".to_string(), Value::String("present".to_string()));

    let mapped = mapper.map_for_module("package", task_1_params).unwrap();
    assert_eq!(mapped.get("name").unwrap().as_str().unwrap(), "git");
    assert_eq!(mapped.get("state").unwrap().as_str().unwrap(), "present");

    // Task 0 from the example plan uses debug module
    let mut task_0_params = HashMap::new();
    task_0_params.insert("msg".to_string(), Value::String("hello world".to_string()));

    let mapped = mapper.map_for_module("debug", task_0_params).unwrap();
    assert_eq!(mapped.get("msg").unwrap().as_str().unwrap(), "hello world");
}
