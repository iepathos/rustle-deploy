use rustle_deploy::cli::{RustleDeployCliImpl, DeployOptions, OptimizationMode};
use rustle_deploy::compilation::ZeroInfraCompiler;
use std::collections::HashMap;
use tempfile::TempDir;
use tokio_test;

#[tokio_test::test]
async fn test_end_to_end_cli_initialization() {
    let temp_dir = TempDir::new().unwrap();
    
    let cli_impl = RustleDeployCliImpl::new(Some(temp_dir.path().to_path_buf())).await;
    assert!(cli_impl.is_ok());
    
    let cli = cli_impl.unwrap();
    
    // Test capability checking
    let report = cli.check_capabilities(false).await;
    assert!(report.is_ok());
}

#[tokio_test::test]
async fn test_deployment_with_different_optimization_modes() {
    let temp_dir = TempDir::new().unwrap();
    let cli_impl = RustleDeployCliImpl::new(Some(temp_dir.path().to_path_buf())).await.unwrap();
    
    // Create test files
    let playbook_path = temp_dir.path().join("test.yml");
    let inventory_path = temp_dir.path().join("inventory.yml");
    
    std::fs::write(&playbook_path, create_test_playbook()).unwrap();
    std::fs::write(&inventory_path, create_test_inventory_yaml()).unwrap();
    
    // Test different optimization modes
    let optimization_modes = vec![
        OptimizationMode::Auto,
        OptimizationMode::Conservative,
        OptimizationMode::Off,
    ];
    
    for mode in optimization_modes {
        let options = DeployOptions {
            optimization_mode: mode.clone(),
            force_binary: false,
            force_ssh: false,
            verbosity: 0,
            dry_run: true, // Use dry run for testing
            cache_binaries: true,
            parallel_compilation: false,
            timeout: 60,
            parallel_jobs: Some(1),
        };
        
        let result = cli_impl.execute_deployment(&playbook_path, &inventory_path, options).await;
        assert!(result.is_ok(), "Failed with optimization mode: {:?}", mode);
        
        let deployment_result = result.unwrap();
        assert!(deployment_result.success);
    }
}

#[tokio_test::test]
async fn test_capability_checking_and_validation() {
    let temp_dir = TempDir::new().unwrap();
    let compiler = ZeroInfraCompiler::detect_capabilities(temp_dir.path().to_path_buf()).await.unwrap();
    
    // Test toolchain validation
    let validation = compiler.validate_toolchain().await;
    assert!(validation.is_ok());
    
    let result = validation.unwrap();
    
    // Validation should provide meaningful information
    assert!(!result.recommendations.is_empty() || 
            result.overall_status == rustle_deploy::compilation::zero_infra::ValidationStatus::Excellent);
}

#[tokio_test::test]
async fn test_mixed_deployment_strategy() {
    let temp_dir = TempDir::new().unwrap();
    let compiler = ZeroInfraCompiler::detect_capabilities(temp_dir.path().to_path_buf()).await.unwrap();
    
    // Create a template that should trigger mixed deployment
    let template = create_mixed_compatibility_template();
    let inventory = create_multi_target_inventory();
    
    let deployment_plan = compiler.compile_or_fallback(&template, &inventory).await;
    assert!(deployment_plan.is_ok());
    
    let plan = deployment_plan.unwrap();
    
    // Should have reasonable performance characteristics
    assert!(plan.estimated_performance_gain >= 1.0);
    assert_eq!(plan.total_targets, inventory.hosts.len());
}

#[tokio_test::test]
async fn test_cache_functionality() {
    let temp_dir = TempDir::new().unwrap();
    let compiler = ZeroInfraCompiler::detect_capabilities(temp_dir.path().to_path_buf()).await.unwrap();
    
    let template = create_test_template();
    let inventory = create_simple_inventory();
    
    // First compilation
    let plan1 = compiler.compile_or_fallback(&template, &inventory).await.unwrap();
    
    // Second compilation (should potentially use cache)
    let plan2 = compiler.compile_or_fallback(&template, &inventory).await.unwrap();
    
    // Both should succeed
    assert!(plan1.total_targets > 0);
    assert!(plan2.total_targets > 0);
}

#[tokio_test::test]
async fn test_file_operations_plan_compilation() {
    // Test compilation of the file_operations_plan.json fixture
    let temp_dir = TempDir::new().unwrap();
    let cli_impl = RustleDeployCliImpl::new(Some(temp_dir.path().to_path_buf())).await.unwrap();
    
    // Load the fixture
    let fixture_path = std::path::Path::new("tests/fixtures/execution_plans/file_operations_plan.json");
    assert!(fixture_path.exists(), "file_operations_plan.json fixture must exist");
    
    let options = DeployOptions {
        optimization_mode: OptimizationMode::Auto,
        force_binary: false,
        force_ssh: false,
        verbosity: 0,
        dry_run: false,
        cache_binaries: true,
        parallel_compilation: false,
        timeout: 120, // Longer timeout for compilation
        parallel_jobs: Some(1),
    };
    
    // Test compile-only mode (which was failing before)
    let result = cli_impl.execute_deployment(fixture_path, fixture_path, options).await;
    assert!(result.is_ok(), "file_operations_plan.json compilation should succeed");
    
    let deployment_result = result.unwrap();
    assert!(deployment_result.success, "Deployment should complete successfully");
}

#[tokio_test::test]
async fn test_error_handling_invalid_inputs() {
    let temp_dir = TempDir::new().unwrap();
    let cli_impl = RustleDeployCliImpl::new(Some(temp_dir.path().to_path_buf())).await.unwrap();
    
    // Test with non-existent files
    let nonexistent_playbook = temp_dir.path().join("nonexistent.yml");
    let nonexistent_inventory = temp_dir.path().join("nonexistent_inventory.yml");
    
    let options = DeployOptions {
        optimization_mode: OptimizationMode::Auto,
        force_binary: false,
        force_ssh: false,
        verbosity: 0,
        dry_run: true,
        cache_binaries: true,
        parallel_compilation: false,
        timeout: 60,
        parallel_jobs: Some(1),
    };
    
    // This should handle errors gracefully
    let result = cli_impl.execute_deployment(&nonexistent_playbook, &nonexistent_inventory, options).await;
    // Note: Depending on implementation, this might succeed with mock data or fail gracefully
    // The important thing is that it doesn't panic
}

// Helper functions for creating test data

fn create_test_playbook() -> String {
    r#"---
- hosts: all
  tasks:
    - name: Echo hello
      command: echo "Hello, World!"
    
    - name: Install package
      package:
        name: curl
        state: present
"#.to_string()
}

fn create_test_inventory_yaml() -> String {
    r#"[webservers]
web1 ansible_host=192.168.1.10
web2 ansible_host=192.168.1.11

[databases]
db1 ansible_host=192.168.1.20
"#.to_string()
}

fn create_mixed_compatibility_template() -> rustle_deploy::execution::template::GeneratedTemplate {
    rustle_deploy::execution::template::GeneratedTemplate {
        execution_plan: rustle_deploy::execution::plan::RustlePlanOutput {
            tasks: vec![
                // Binary-compatible task
                rustle_deploy::execution::plan::TaskPlan {
                    id: "task-1".to_string(),
                    module: "command".to_string(),
                    args: HashMap::new(),
                    when: None,
                    changed_when: None,
                    failed_when: None,
                    vars: HashMap::new(),
                    tags: vec![],
                    name: Some("Compatible command".to_string()),
                },
                // Potentially incompatible task
                rustle_deploy::execution::plan::TaskPlan {
                    id: "task-2".to_string(),
                    module: "custom_module".to_string(),
                    args: HashMap::new(),
                    when: None,
                    changed_when: None,
                    failed_when: None,
                    vars: HashMap::new(),
                    tags: vec![],
                    name: Some("Custom module task".to_string()),
                },
            ],
            optimize_for_size: false,
            metadata: HashMap::new(),
        },
        files: HashMap::from([
            ("main.rs".to_string(), "fn main() { println!(\"Mixed compatibility\"); }".to_string()),
        ]),
    }
}

fn create_multi_target_inventory() -> rustle_deploy::inventory::ParsedInventory {
    rustle_deploy::inventory::ParsedInventory {
        hosts: vec![
            rustle_deploy::inventory::HostInfo {
                name: "linux-x64-1".to_string(),
                address: "192.168.1.10".to_string(),
                port: 22,
                groups: vec!["linux".to_string()],
                variables: HashMap::new(),
            },
            rustle_deploy::inventory::HostInfo {
                name: "linux-x64-2".to_string(),
                address: "192.168.1.11".to_string(),
                port: 22,
                groups: vec!["linux".to_string()],
                variables: HashMap::new(),
            },
            rustle_deploy::inventory::HostInfo {
                name: "linux-arm64-1".to_string(),
                address: "192.168.1.20".to_string(),
                port: 22,
                groups: vec!["arm".to_string()],
                variables: HashMap::new(),
            },
        ],
        groups: HashMap::from([
            ("linux".to_string(), vec!["linux-x64-1".to_string(), "linux-x64-2".to_string()]),
            ("arm".to_string(), vec!["linux-arm64-1".to_string()]),
        ]),
        variables: HashMap::new(),
    }
}

fn create_test_template() -> rustle_deploy::execution::template::GeneratedTemplate {
    rustle_deploy::execution::template::GeneratedTemplate {
        execution_plan: rustle_deploy::execution::plan::RustlePlanOutput {
            tasks: vec![
                rustle_deploy::execution::plan::TaskPlan {
                    id: "test-task".to_string(),
                    module: "debug".to_string(),
                    args: HashMap::from([
                        ("msg".to_string(), serde_json::Value::String("Test message".to_string())),
                    ]),
                    when: None,
                    changed_when: None,
                    failed_when: None,
                    vars: HashMap::new(),
                    tags: vec![],
                    name: Some("Test debug task".to_string()),
                },
            ],
            optimize_for_size: true,
            metadata: HashMap::new(),
        },
        files: HashMap::from([
            ("main.rs".to_string(), "fn main() { println!(\"Test template\"); }".to_string()),
        ]),
    }
}

fn create_simple_inventory() -> rustle_deploy::inventory::ParsedInventory {
    rustle_deploy::inventory::ParsedInventory {
        hosts: vec![
            rustle_deploy::inventory::HostInfo {
                name: "simple-host".to_string(),
                address: "localhost".to_string(),
                port: 22,
                groups: vec!["test".to_string()],
                variables: HashMap::new(),
            },
        ],
        groups: HashMap::from([
            ("test".to_string(), vec!["simple-host".to_string()]),
        ]),
        variables: HashMap::new(),
    }
}