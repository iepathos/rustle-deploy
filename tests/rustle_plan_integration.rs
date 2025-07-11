use rustle_deploy::execution::{validate_rustle_plan_json, RustlePlanConverter};
use std::fs;

#[test]
fn test_parse_example_rustle_plan_output() {
    let content = fs::read_to_string("example_rustle_plan_output.json")
        .expect("Failed to read example rustle plan output file");

    let result = validate_rustle_plan_json(&content);
    assert!(
        result.is_ok(),
        "Failed to parse example rustle plan output: {result:?}"
    );

    let rustle_plan = result.unwrap();
    assert_eq!(rustle_plan.total_tasks, 3);
    assert_eq!(rustle_plan.hosts.len(), 1);
    assert_eq!(rustle_plan.hosts[0], "localhost");
}

#[test]
fn test_convert_rustle_plan_to_execution_plan() {
    let content = fs::read_to_string("example_rustle_plan_output.json")
        .expect("Failed to read example rustle plan output file");

    let rustle_plan = validate_rustle_plan_json(&content).expect("Failed to parse rustle plan");

    let converter = RustlePlanConverter::new();
    let result = converter.convert_to_execution_plan(&rustle_plan);

    assert!(result.is_ok(), "Failed to convert rustle plan: {result:?}");

    let execution_plan = result.unwrap();
    assert_eq!(execution_plan.tasks.len(), 3);
}
