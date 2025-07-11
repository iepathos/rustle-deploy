//! Integration tests for execution modules

use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;

use rustle_deploy::modules::{
    ExecutionContext, HostInfo, ModuleArgs, ModuleRegistry, SpecialParameters,
};

#[tokio::test]
async fn test_debug_module_basic() {
    let registry = ModuleRegistry::with_core_modules();

    let args = ModuleArgs {
        args: {
            let mut map = HashMap::new();
            map.insert("msg".to_string(), json!("Hello from test!"));
            map
        },
        special: SpecialParameters::default(),
    };

    let context = ExecutionContext {
        facts: HashMap::new(),
        variables: HashMap::new(),
        host_info: HostInfo::detect(),
        working_directory: PathBuf::from("/tmp"),
        environment: std::env::vars().collect(),
        check_mode: false,
        diff_mode: false,
        verbosity: 0,
    };

    let result = registry.execute_module("debug", &args, &context).await;
    assert!(result.is_ok());

    let module_result = result.unwrap();
    assert!(!module_result.changed);
    assert!(!module_result.failed);
    assert_eq!(module_result.msg, Some("Hello from test!".to_string()));
    assert_eq!(module_result.rc, Some(0));
}

#[tokio::test]
async fn test_debug_module_variable() {
    let registry = ModuleRegistry::with_core_modules();

    let args = ModuleArgs {
        args: {
            let mut map = HashMap::new();
            map.insert("var".to_string(), json!("test_var"));
            map
        },
        special: SpecialParameters::default(),
    };

    let context = ExecutionContext {
        facts: HashMap::new(),
        variables: {
            let mut vars = HashMap::new();
            vars.insert("test_var".to_string(), json!("test_value"));
            vars
        },
        host_info: HostInfo::detect(),
        working_directory: PathBuf::from("/tmp"),
        environment: std::env::vars().collect(),
        check_mode: false,
        diff_mode: false,
        verbosity: 0,
    };

    let result = registry.execute_module("debug", &args, &context).await;
    assert!(result.is_ok());

    let module_result = result.unwrap();
    assert!(!module_result.changed);
    assert!(!module_result.failed);
    assert!(module_result.msg.unwrap().contains("test_var"));
}

#[tokio::test]
async fn test_command_module_basic() {
    let registry = ModuleRegistry::with_core_modules();

    let args = ModuleArgs {
        args: {
            let mut map = HashMap::new();
            map.insert("_raw_params".to_string(), json!("echo 'Hello World'"));
            map
        },
        special: SpecialParameters::default(),
    };

    let context = ExecutionContext {
        facts: HashMap::new(),
        variables: HashMap::new(),
        host_info: HostInfo::detect(),
        working_directory: PathBuf::from("/tmp"),
        environment: std::env::vars().collect(),
        check_mode: false,
        diff_mode: false,
        verbosity: 0,
    };

    let result = registry.execute_module("command", &args, &context).await;
    assert!(result.is_ok());

    let module_result = result.unwrap();
    assert!(module_result.changed);
    assert!(!module_result.failed);
    assert!(module_result.stdout.unwrap().contains("Hello World"));
    assert_eq!(module_result.rc, Some(0));
}

#[tokio::test]
async fn test_command_module_check_mode() {
    let registry = ModuleRegistry::with_core_modules();

    let args = ModuleArgs {
        args: {
            let mut map = HashMap::new();
            map.insert("_raw_params".to_string(), json!("echo 'Hello World'"));
            map
        },
        special: SpecialParameters::default(),
    };

    let context = ExecutionContext {
        facts: HashMap::new(),
        variables: HashMap::new(),
        host_info: HostInfo::detect(),
        working_directory: PathBuf::from("/tmp"),
        environment: std::env::vars().collect(),
        check_mode: true,
        diff_mode: false,
        verbosity: 0,
    };

    let result = registry.execute_module("command", &args, &context).await;
    assert!(result.is_ok());

    let module_result = result.unwrap();
    assert!(module_result.changed);
    assert!(!module_result.failed);
    assert_eq!(module_result.msg, Some("Command would run".to_string()));
    assert_eq!(module_result.rc, None);
}

#[tokio::test]
async fn test_command_module_validation_error() {
    let registry = ModuleRegistry::with_core_modules();

    let args = ModuleArgs {
        args: HashMap::new(), // No command specified
        special: SpecialParameters::default(),
    };

    let context = ExecutionContext {
        facts: HashMap::new(),
        variables: HashMap::new(),
        host_info: HostInfo::detect(),
        working_directory: PathBuf::from("/tmp"),
        environment: std::env::vars().collect(),
        check_mode: false,
        diff_mode: false,
        verbosity: 0,
    };

    let result = registry.execute_module("command", &args, &context).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_module_registry_list_modules() {
    let registry = ModuleRegistry::with_core_modules();
    let modules = registry.list_modules();

    assert!(modules.contains(&"debug"));
    assert!(modules.contains(&"command"));
    assert!(modules.contains(&"package"));
    assert!(modules.contains(&"service"));
}

#[tokio::test]
async fn test_module_registry_get_module() {
    let registry = ModuleRegistry::with_core_modules();

    let debug_module = registry.get_module("debug");
    assert!(debug_module.is_some());
    assert_eq!(debug_module.unwrap().name(), "debug");

    let nonexistent_module = registry.get_module("nonexistent");
    assert!(nonexistent_module.is_none());
}
