use rustle_deploy::execution::{ExecutionPlanParser, ParseError, PlanFormat};
use std::fs;

#[test]
fn test_parse_simple_json_plan() {
    let parser = ExecutionPlanParser::new();
    let content = fs::read_to_string("tests/fixtures/execution_plans/simple_plan.json")
        .expect("Failed to read test fixture");

    let result = parser.parse(&content, PlanFormat::Json);
    assert!(
        result.is_ok(),
        "Failed to parse simple JSON plan: {:?}",
        result.err()
    );

    let plan = result.unwrap();
    assert_eq!(plan.metadata.plan_id, "test-plan-001");
    assert_eq!(plan.tasks.len(), 1);
    assert_eq!(plan.tasks[0].name, "Install package");
}

#[test]
fn test_parse_auto_detect_json() {
    let parser = ExecutionPlanParser::new();
    let content = fs::read_to_string("tests/fixtures/execution_plans/simple_plan.json")
        .expect("Failed to read test fixture");

    let result = parser.parse(&content, PlanFormat::Auto);
    assert!(
        result.is_ok(),
        "Failed to auto-detect JSON format: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_invalid_json() {
    let parser = ExecutionPlanParser::new();
    let invalid_content = "{ invalid json";

    let result = parser.parse(invalid_content, PlanFormat::Json);
    assert!(result.is_err());

    match result.unwrap_err() {
        ParseError::InvalidJson { .. } => {} // Expected
        other => panic!("Expected InvalidJson error, got: {other:?}"),
    }
}

#[test]
fn test_validate_plan() {
    let parser = ExecutionPlanParser::new();
    let content = fs::read_to_string("tests/fixtures/execution_plans/simple_plan.json")
        .expect("Failed to read test fixture");

    let plan = parser.parse(&content, PlanFormat::Json).unwrap();
    let result = parser.validate(&plan);
    assert!(result.is_ok(), "Plan validation failed: {:?}", result.err());
}

#[test]
fn test_extract_deployment_targets() {
    let parser = ExecutionPlanParser::new();
    let content = fs::read_to_string("tests/fixtures/execution_plans/simple_plan.json")
        .expect("Failed to read test fixture");

    let plan = parser.parse(&content, PlanFormat::Json).unwrap();
    let targets = parser.extract_deployment_targets(&plan).unwrap();

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].host, "test.example.com");
    assert_eq!(targets[0].target_path, "/tmp/rustle-runner");
}

#[test]
fn test_compute_execution_order() {
    let parser = ExecutionPlanParser::new();
    let content = fs::read_to_string("tests/fixtures/execution_plans/simple_plan.json")
        .expect("Failed to read test fixture");

    let plan = parser.parse(&content, PlanFormat::Json).unwrap();
    let order = parser.compute_execution_order(&plan).unwrap();

    assert_eq!(order.len(), 1);
    assert_eq!(order[0], "task-001");
}
