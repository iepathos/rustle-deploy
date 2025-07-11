use rustle_deploy::deploy::DeploymentManager;
use rustle_deploy::execution::{ExecutionPlanParser, PlanFormat};
use rustle_deploy::types::DeploymentConfig;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_end_to_end_deployment_plan_creation() {
    let temp_dir = TempDir::new().unwrap();
    let config = DeploymentConfig {
        cache_dir: temp_dir.path().to_path_buf(),
        output_dir: temp_dir.path().to_path_buf(),
        parallel_jobs: 4,
        default_timeout_secs: 300,
        verify_deployments: true,
        compression: true,
        strip_symbols: true,
        binary_size_limit_mb: 100,
    };

    let manager = DeploymentManager::new(config);
    let content = fs::read_to_string("tests/fixtures/execution_plans/simple_plan.json")
        .expect("Failed to read test fixture");

    let result = manager
        .create_deployment_plan(&content, PlanFormat::Json)
        .await;
    assert!(
        result.is_ok(),
        "Failed to create deployment plan: {:?}",
        result.err()
    );

    let plan = result.unwrap();
    assert_eq!(plan.deployment_targets.len(), 1);
    assert_eq!(plan.binary_compilations.len(), 1);
    assert!(!plan.metadata.deployment_id.is_empty());
}

#[test]
fn test_execution_plan_serialization_roundtrip() {
    let parser = ExecutionPlanParser::new();
    let content = fs::read_to_string("tests/fixtures/execution_plans/simple_plan.json")
        .expect("Failed to read test fixture");

    // Parse the plan
    let plan = parser.parse(&content, PlanFormat::Json).unwrap();

    // Serialize it back to JSON
    let serialized = serde_json::to_string(&plan).unwrap();

    // Parse it again
    let reparsed = parser.parse(&serialized, PlanFormat::Json).unwrap();

    // They should be equivalent
    assert_eq!(plan.metadata.plan_id, reparsed.metadata.plan_id);
    assert_eq!(plan.tasks.len(), reparsed.tasks.len());
    assert_eq!(plan.tasks[0].name, reparsed.tasks[0].name);
}
