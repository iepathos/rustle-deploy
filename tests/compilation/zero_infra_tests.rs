use rustle_deploy::compilation::{ZeroInfraCompiler, CompilationCapabilities};
use rustle_deploy::execution::template::GeneratedTemplate;
use rustle_deploy::inventory::ParsedInventory;
use std::collections::HashMap;
use tempfile::TempDir;
use tokio_test;

#[tokio_test::test]
async fn test_capability_detection() {
    let temp_dir = TempDir::new().unwrap();
    
    // Test basic capability detection
    let capabilities = CompilationCapabilities::detect_basic().await;
    assert!(capabilities.is_ok());
    
    let caps = capabilities.unwrap();
    assert!(!caps.native_target.is_empty());
    assert!(!caps.available_targets.is_empty());
}

#[tokio_test::test]
async fn test_compiler_initialization() {
    let temp_dir = TempDir::new().unwrap();
    
    let compiler = ZeroInfraCompiler::detect_capabilities(temp_dir.path().to_path_buf()).await;
    assert!(compiler.is_ok());
    
    let compiler = compiler.unwrap();
    assert!(!compiler.get_available_targets().is_empty());
}

#[tokio_test::test]
async fn test_deployment_plan_creation() {
    let temp_dir = TempDir::new().unwrap();
    let compiler = ZeroInfraCompiler::detect_capabilities(temp_dir.path().to_path_buf()).await.unwrap();
    
    // Create a mock template and inventory
    let template = create_mock_template();
    let inventory = create_mock_inventory();
    
    let deployment_plan = compiler.compile_or_fallback(&template, &inventory).await;
    assert!(deployment_plan.is_ok());
    
    let plan = deployment_plan.unwrap();
    assert!(plan.total_targets > 0);
}

#[tokio_test::test]
async fn test_target_fallback_detection() {
    let temp_dir = TempDir::new().unwrap();
    let compiler = ZeroInfraCompiler::detect_capabilities(temp_dir.path().to_path_buf()).await.unwrap();
    
    // Test with unsupported target
    let unsupported_target = "unknown-unknown-unknown";
    assert!(compiler.requires_fallback(unsupported_target));
    
    // Test with native target (should be supported)
    let native_target = std::env::consts::TARGET;
    assert!(!compiler.requires_fallback(native_target));
}

#[tokio_test::test]
async fn test_toolchain_validation() {
    let temp_dir = TempDir::new().unwrap();
    let compiler = ZeroInfraCompiler::detect_capabilities(temp_dir.path().to_path_buf()).await.unwrap();
    
    let validation = compiler.validate_toolchain().await;
    assert!(validation.is_ok());
    
    let result = validation.unwrap();
    // At minimum, we should have some status (even if components are missing)
    assert!(!result.issues.is_empty() || !result.recommendations.is_empty() || 
            result.overall_status != rustle_deploy::compilation::zero_infra::ValidationStatus::Failed);
}

// Helper functions for creating mock data

fn create_mock_template() -> GeneratedTemplate {
    GeneratedTemplate {
        execution_plan: rustle_deploy::execution::plan::RustlePlanOutput {
            tasks: vec![
                rustle_deploy::execution::plan::TaskPlan {
                    id: "test-task-1".to_string(),
                    module: "command".to_string(),
                    args: HashMap::new(),
                    when: None,
                    changed_when: None,
                    failed_when: None,
                    vars: HashMap::new(),
                    tags: vec![],
                    name: Some("Test task".to_string()),
                },
            ],
            optimize_for_size: false,
            metadata: HashMap::new(),
        },
        files: HashMap::from([
            ("main.rs".to_string(), "fn main() { println!(\"Hello, world!\"); }".to_string()),
        ]),
    }
}

fn create_mock_inventory() -> ParsedInventory {
    ParsedInventory {
        hosts: vec![
            rustle_deploy::inventory::HostInfo {
                name: "test-host-1".to_string(),
                address: "192.168.1.100".to_string(),
                port: 22,
                groups: vec!["webservers".to_string()],
                variables: HashMap::new(),
            },
            rustle_deploy::inventory::HostInfo {
                name: "test-host-2".to_string(),
                address: "192.168.1.101".to_string(),
                port: 22,
                groups: vec!["webservers".to_string()],
                variables: HashMap::new(),
            },
        ],
        groups: HashMap::from([
            ("webservers".to_string(), vec!["test-host-1".to_string(), "test-host-2".to_string()]),
        ]),
        variables: HashMap::new(),
    }
}