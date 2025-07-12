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

// Ansible Features Tests
#[test]
fn test_parse_ansible_demo_plan() {
    let parser = ExecutionPlanParser::new();
    let content = fs::read_to_string("tests/fixtures/execution_plans/ansible_demo_plan.json")
        .expect("Failed to read ansible demo plan fixture");

    let result = parser.parse(&content, PlanFormat::Json);
    assert!(
        result.is_ok(),
        "Failed to parse ansible demo plan: {:?}",
        result.err()
    );

    let plan = result.unwrap();
    assert_eq!(plan.metadata.plan_id, "ansible-demo-plan-001");
    assert_eq!(plan.tasks.len(), 3);

    // Verify setup module task exists
    let setup_task = plan
        .tasks
        .iter()
        .find(|t| t.module == "setup")
        .expect("Should have setup task");
    assert_eq!(setup_task.name, "Gather system facts");

    // Verify template task exists
    let template_task = plan
        .tasks
        .iter()
        .find(|t| t.module == "template")
        .expect("Should have template task");
    match template_task.task_type {
        rustle_deploy::execution::TaskType::Template => {}
        _ => panic!("Expected Template task type"),
    }
    assert!(template_task.args.contains_key("variables"));

    // Verify copy task exists
    let copy_task = plan
        .tasks
        .iter()
        .find(|t| t.module == "copy")
        .expect("Should have copy task");
    match copy_task.task_type {
        rustle_deploy::execution::TaskType::Copy => {}
        _ => panic!("Expected Copy task type"),
    }
}

#[test]
fn test_parse_minimal_ansible_plan() {
    let parser = ExecutionPlanParser::new();
    let content = fs::read_to_string("tests/fixtures/execution_plans/minimal_ansible_plan.json")
        .expect("Failed to read minimal ansible plan fixture");

    let result = parser.parse(&content, PlanFormat::Json);
    assert!(
        result.is_ok(),
        "Failed to parse minimal ansible plan: {:?}",
        result.err()
    );

    let plan = result.unwrap();
    assert_eq!(plan.metadata.plan_id, "minimal-ansible-plan-001");
    assert_eq!(plan.tasks.len(), 3);

    // Verify setup module task exists
    let setup_task = plan
        .tasks
        .iter()
        .find(|t| t.module == "setup")
        .expect("Should have setup task");
    assert_eq!(setup_task.name, "Gather system facts");

    // Verify template task exists
    let template_task = plan
        .tasks
        .iter()
        .find(|t| t.module == "template")
        .expect("Should have template task");
    assert!(template_task.args.contains_key("variables"));

    // Verify copy task with condition exists
    let copy_task = plan
        .tasks
        .iter()
        .find(|t| t.module == "copy")
        .expect("Should have copy task");
    assert!(!copy_task.conditions.is_empty());
    assert_eq!(copy_task.conditions[0].variable, "ansible_system");
    assert_eq!(copy_task.conditions[0].value, "Linux");
}

#[test]
fn test_ansible_features_comprehensive() {
    let parser = ExecutionPlanParser::new();

    // Test with working plans
    let working_plans = [
        "tests/fixtures/execution_plans/minimal_ansible_plan.json",
        "tests/fixtures/execution_plans/ansible_demo_plan.json",
    ];

    for plan_path in &working_plans {
        let content = fs::read_to_string(plan_path)
            .unwrap_or_else(|_| panic!("Failed to read {}", plan_path));

        let plan = parser
            .parse(&content, PlanFormat::Json)
            .unwrap_or_else(|e| panic!("Failed to parse {}: {:?}", plan_path, e));

        let validation_result = parser.validate(&plan);
        assert!(
            validation_result.is_ok(),
            "Plan validation failed for {}: {:?}",
            plan_path,
            validation_result.err()
        );
    }
}
