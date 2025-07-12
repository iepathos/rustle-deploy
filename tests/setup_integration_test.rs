//! Integration test for setup module

use rustle_deploy::modules::{
    interface::{ExecutionContext, HostInfo, ModuleArgs},
    registry::ModuleRegistry,
};
use std::collections::HashMap;
use std::path::PathBuf;

#[tokio::test]
async fn test_setup_module_registration() {
    let registry = ModuleRegistry::with_core_modules();

    // Check that setup module is registered
    let modules = registry.list_modules();
    assert!(modules.contains(&"setup"));

    // Get the setup module
    let setup_module = registry.get_module("setup");
    assert!(setup_module.is_some());

    let setup_module = setup_module.unwrap();
    assert_eq!(setup_module.name(), "setup");
    assert_eq!(setup_module.version(), "1.0.0");
}

#[tokio::test]
async fn test_setup_module_execution_via_registry() {
    let registry = ModuleRegistry::with_core_modules();

    let args = ModuleArgs {
        args: HashMap::new(),
        special: Default::default(),
    };

    let context = ExecutionContext {
        facts: HashMap::new(),
        variables: HashMap::new(),
        host_info: HostInfo::detect(),
        working_directory: PathBuf::from("/tmp"),
        environment: HashMap::new(),
        check_mode: false,
        diff_mode: false,
        verbosity: 0,
    };

    let result = registry.execute_module("setup", &args, &context).await;
    assert!(result.is_ok());

    let module_result = result.unwrap();
    assert!(!module_result.failed);
    assert!(!module_result.changed);
    assert!(!module_result.ansible_facts.is_empty());

    // Verify essential facts are present
    assert!(module_result.ansible_facts.contains_key("ansible_system"));
    assert!(module_result.ansible_facts.contains_key("ansible_hostname"));
    assert!(module_result
        .ansible_facts
        .contains_key("ansible_architecture"));
}
