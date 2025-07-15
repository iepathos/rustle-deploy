use anyhow::Result;
use rustle_deploy::compilation::target_detection::TargetDetector;
use rustle_deploy::execution::rustle_plan::RustlePlanOutput;
use rustle_deploy::types::compilation::OptimizationLevel;
use std::fs;

#[test]
fn test_parse_new_plan_format_with_compilation_requirements() -> Result<()> {
    // Load the new fixture
    let plan_content = fs::read_to_string(
        "tests/fixtures/execution_plans/file_operations_playbook_with_facts_plan.json",
    )?;

    // Parse the plan
    let plan: RustlePlanOutput = serde_json::from_str(&plan_content)?;

    // Verify metadata
    assert_eq!(plan.metadata.rustle_plan_version, "0.1.0");
    assert_eq!(
        plan.metadata.playbook_hash,
        "8d1386d0f7759457a0b5efea5de4816e"
    );

    // Verify binary deployments
    assert_eq!(plan.binary_deployments.len(), 1);
    let deployment = &plan.binary_deployments[0];

    // Verify compilation requirements
    assert_eq!(deployment.compilation_requirements.target_arch, "aarch64");
    assert_eq!(deployment.compilation_requirements.target_os, "darwin");
    assert_eq!(deployment.compilation_requirements.rust_version, "1.70.0");
    assert!(!deployment.compilation_requirements.cross_compilation);
    assert!(deployment.compilation_requirements.static_linking);

    // Verify embedded data
    assert!(!deployment.embedded_data.execution_plan.is_empty());
    assert_eq!(deployment.embedded_data.static_files.len(), 1);
    assert!(deployment
        .embedded_data
        .facts_required
        .contains(&"ansible_user_gid".to_string()));
    assert!(deployment
        .embedded_data
        .facts_required
        .contains(&"ansible_user_uid".to_string()));

    // Verify tasks
    assert_eq!(deployment.tasks.len(), 5);
    assert_eq!(deployment.modules.len(), 5);

    // Verify plays
    assert_eq!(plan.plays.len(), 1);
    let play = &plan.plays[0];
    assert_eq!(play.name, "Comprehensive file operations playbook");
    assert_eq!(play.batches.len(), 1);

    // Verify task details
    let batch = &play.batches[0];
    assert_eq!(batch.tasks.len(), 5);
    let first_task = &batch.tasks[0];
    assert_eq!(first_task.name, "Create base directory structure");
    assert_eq!(first_task.module, "file");

    Ok(())
}

#[test]
fn test_embedded_execution_plan_parsing() -> Result<()> {
    // Load the new fixture
    let plan_content = fs::read_to_string(
        "tests/fixtures/execution_plans/file_operations_playbook_with_facts_plan.json",
    )?;

    let plan: RustlePlanOutput = serde_json::from_str(&plan_content)?;
    let deployment = &plan.binary_deployments[0];

    // Parse the embedded execution plan
    let embedded_plan = deployment.parse_execution_plan()?;

    // Verify it's valid JSON
    assert!(embedded_plan.is_object());

    // Verify key fields
    assert_eq!(embedded_plan["group_id"], "group_0");
    assert!(embedded_plan["hosts"].is_array());
    assert!(embedded_plan["tasks"].is_array());

    Ok(())
}

#[test]
fn test_target_spec_from_compilation_requirements() -> Result<()> {
    // Load the new fixture
    let plan_content = fs::read_to_string(
        "tests/fixtures/execution_plans/file_operations_playbook_with_facts_plan.json",
    )?;

    let plan: RustlePlanOutput = serde_json::from_str(&plan_content)?;
    let deployment = &plan.binary_deployments[0];

    // Create target detector and generate target spec from requirements
    let target_detector = TargetDetector::new();
    let target_spec = target_detector.create_target_spec_from_requirements(
        &deployment.compilation_requirements,
        OptimizationLevel::Release,
    )?;

    // Verify target spec is created correctly
    assert_eq!(target_spec.target_triple, "aarch64-apple-darwin");

    // Since cross_compilation is false in the fixture, it should not force zigbuild
    assert!(!deployment.compilation_requirements.cross_compilation);

    Ok(())
}
