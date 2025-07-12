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

#[test]
fn test_file_module_basic_params() {
    let mapper = parameter_mapping::ParameterMapper::new();
    let mut params = HashMap::new();
    params.insert("path".to_string(), Value::String("/tmp/test".to_string()));
    params.insert("state".to_string(), Value::String("directory".to_string()));
    params.insert("mode".to_string(), Value::String("0755".to_string()));

    let mapped = mapper.map_for_module("file", params).unwrap();

    assert_eq!(mapped.get("path").unwrap().as_str().unwrap(), "/tmp/test");
    assert_eq!(mapped.get("state").unwrap().as_str().unwrap(), "directory");
    assert_eq!(mapped.get("mode").unwrap().as_str().unwrap(), "0755");
}

#[test]
fn test_file_module_link_operation() {
    let mapper = parameter_mapping::ParameterMapper::new();
    let mut params = HashMap::new();
    params.insert("dest".to_string(), Value::String("/tmp/link".to_string()));
    params.insert("src".to_string(), Value::String("/tmp/target".to_string()));
    params.insert("state".to_string(), Value::String("link".to_string()));

    let mapped = mapper.map_for_module("file", params).unwrap();

    // dest should map to path for file module
    assert_eq!(mapped.get("path").unwrap().as_str().unwrap(), "/tmp/link");
    assert_eq!(mapped.get("src").unwrap().as_str().unwrap(), "/tmp/target");
    assert_eq!(mapped.get("state").unwrap().as_str().unwrap(), "link");
    assert_eq!(mapped.get("dest").unwrap().as_str().unwrap(), "/tmp/link");
}

#[test]
fn test_file_module_missing_path() {
    let mapper = parameter_mapping::ParameterMapper::new();
    let mut params = HashMap::new();
    params.insert("mode".to_string(), Value::String("0755".to_string()));

    let result = mapper.map_for_module("file", params);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Missing required parameter: path"));
}

#[test]
fn test_file_module_link_missing_src() {
    let mapper = parameter_mapping::ParameterMapper::new();
    let mut params = HashMap::new();
    params.insert("path".to_string(), Value::String("/tmp/link".to_string()));
    params.insert("state".to_string(), Value::String("link".to_string()));

    let result = mapper.map_for_module("file", params);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("src (required for link state)"));
}

#[test]
fn test_copy_module_basic_params() {
    let mapper = parameter_mapping::ParameterMapper::new();
    let mut params = HashMap::new();
    params.insert("src".to_string(), Value::String("/source/file".to_string()));
    params.insert("dest".to_string(), Value::String("/dest/file".to_string()));
    params.insert("mode".to_string(), Value::String("0644".to_string()));
    params.insert("backup".to_string(), Value::Bool(true));

    let mapped = mapper.map_for_module("copy", params).unwrap();

    assert_eq!(mapped.get("src").unwrap().as_str().unwrap(), "/source/file");
    assert_eq!(mapped.get("dest").unwrap().as_str().unwrap(), "/dest/file");
    assert_eq!(mapped.get("mode").unwrap().as_str().unwrap(), "0644");
    assert_eq!(mapped.get("backup").unwrap().as_bool().unwrap(), true);
}

#[test]
fn test_copy_module_missing_src() {
    let mapper = parameter_mapping::ParameterMapper::new();
    let mut params = HashMap::new();
    params.insert("dest".to_string(), Value::String("/dest/file".to_string()));

    let result = mapper.map_for_module("copy", params);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Missing required parameter: src"));
}

#[test]
fn test_copy_module_missing_dest() {
    let mapper = parameter_mapping::ParameterMapper::new();
    let mut params = HashMap::new();
    params.insert("src".to_string(), Value::String("/source/file".to_string()));

    let result = mapper.map_for_module("copy", params);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Missing required parameter: dest"));
}

#[test]
fn test_copy_module_all_parameters() {
    let mapper = parameter_mapping::ParameterMapper::new();
    let mut params = HashMap::new();
    params.insert("src".to_string(), Value::String("/source/file".to_string()));
    params.insert("dest".to_string(), Value::String("/dest/file".to_string()));
    params.insert("mode".to_string(), Value::String("0644".to_string()));
    params.insert("owner".to_string(), Value::String("user".to_string()));
    params.insert("group".to_string(), Value::String("group".to_string()));
    params.insert("backup".to_string(), Value::Bool(true));
    params.insert("force".to_string(), Value::Bool(false));
    params.insert("preserve".to_string(), Value::Bool(true));
    params.insert(
        "validate".to_string(),
        Value::String("test -f %s".to_string()),
    );

    let mapped = mapper.map_for_module("copy", params).unwrap();

    assert_eq!(mapped.get("src").unwrap().as_str().unwrap(), "/source/file");
    assert_eq!(mapped.get("dest").unwrap().as_str().unwrap(), "/dest/file");
    assert_eq!(mapped.get("mode").unwrap().as_str().unwrap(), "0644");
    assert_eq!(mapped.get("owner").unwrap().as_str().unwrap(), "user");
    assert_eq!(mapped.get("group").unwrap().as_str().unwrap(), "group");
    assert_eq!(mapped.get("backup").unwrap().as_bool().unwrap(), true);
    assert_eq!(mapped.get("force").unwrap().as_bool().unwrap(), false);
    assert_eq!(mapped.get("preserve").unwrap().as_bool().unwrap(), true);
    assert_eq!(
        mapped.get("validate").unwrap().as_str().unwrap(),
        "test -f %s"
    );
}

#[test]
fn test_real_world_file_operations_scenario() {
    // Test the actual parameters from the file_operations_plan.json we fixed
    let mapper = parameter_mapping::ParameterMapper::new();

    // Test task_0: directory creation
    let mut task_0_params = HashMap::new();
    task_0_params.insert(
        "path".to_string(),
        Value::String("/tmp/rustle_file_test".to_string()),
    );
    task_0_params.insert("recurse".to_string(), Value::Bool(true));
    task_0_params.insert("state".to_string(), Value::String("directory".to_string()));
    task_0_params.insert("mode".to_string(), Value::String("0755".to_string()));

    let mapped = mapper.map_for_module("file", task_0_params).unwrap();
    assert_eq!(
        mapped.get("path").unwrap().as_str().unwrap(),
        "/tmp/rustle_file_test"
    );
    assert_eq!(mapped.get("state").unwrap().as_str().unwrap(), "directory");

    // Test task_2: copy operation
    let mut task_2_params = HashMap::new();
    task_2_params.insert(
        "dest".to_string(),
        Value::String("/tmp/rustle_file_test/config/app.conf".to_string()),
    );
    task_2_params.insert(
        "src".to_string(),
        Value::String("tests/fixtures/files/test_files/sample.conf".to_string()),
    );
    task_2_params.insert("mode".to_string(), Value::String("0644".to_string()));
    task_2_params.insert("backup".to_string(), Value::Bool(true));

    let mapped = mapper.map_for_module("copy", task_2_params).unwrap();
    assert_eq!(
        mapped.get("src").unwrap().as_str().unwrap(),
        "tests/fixtures/files/test_files/sample.conf"
    );
    assert_eq!(
        mapped.get("dest").unwrap().as_str().unwrap(),
        "/tmp/rustle_file_test/config/app.conf"
    );

    // Test task_4: link operation
    let mut task_4_params = HashMap::new();
    task_4_params.insert("state".to_string(), Value::String("link".to_string()));
    task_4_params.insert(
        "dest".to_string(),
        Value::String("/tmp/rustle_file_test/current.conf".to_string()),
    );
    task_4_params.insert(
        "src".to_string(),
        Value::String("/tmp/rustle_file_test/config/app.conf".to_string()),
    );

    let mapped = mapper.map_for_module("file", task_4_params).unwrap();
    assert_eq!(
        mapped.get("path").unwrap().as_str().unwrap(),
        "/tmp/rustle_file_test/current.conf"
    ); // dest mapped to path
    assert_eq!(
        mapped.get("src").unwrap().as_str().unwrap(),
        "/tmp/rustle_file_test/config/app.conf"
    );
    assert_eq!(mapped.get("state").unwrap().as_str().unwrap(), "link");
}
