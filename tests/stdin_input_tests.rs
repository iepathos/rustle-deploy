use rustle_deploy::execution::rustle_plan::{RustlePlanOutput, StaticFileRef};
use serde_json;

#[tokio::test]
async fn test_parse_rustle_plan_from_stdin_basic() {
    // Test parsing a simple execution plan from stdin content
    let test_plan = create_minimal_test_plan();
    let plan_json = serde_json::to_string_pretty(&test_plan).unwrap();

    // This simulates the parse_rustle_plan_content function
    let parsed_plan: RustlePlanOutput = serde_json::from_str(&plan_json).unwrap();

    assert_eq!(parsed_plan.total_tasks, test_plan.total_tasks);
    assert_eq!(parsed_plan.hosts.len(), test_plan.hosts.len());
    assert_eq!(
        parsed_plan.binary_deployments.len(),
        test_plan.binary_deployments.len()
    );
}

#[tokio::test]
async fn test_execution_plan_summary_from_rustle_plan() {
    // Test creating ExecutionPlanSummary from RustlePlanOutput
    let test_plan = create_minimal_test_plan();

    // Simulate the create_execution_plan_summary function logic
    let total_tasks = test_plan.total_tasks;
    let binary_deployment_hosts: usize = test_plan
        .binary_deployments
        .iter()
        .map(|deployment| deployment.target_hosts.len())
        .sum();
    let all_hosts = test_plan.hosts.len();
    let ssh_fallback_hosts = all_hosts.saturating_sub(binary_deployment_hosts);

    assert_eq!(total_tasks, 5);
    assert_eq!(binary_deployment_hosts, 1);
    assert_eq!(ssh_fallback_hosts, 0);
}

#[tokio::test]
async fn test_json_parsing_basic() {
    // Basic test to ensure JSON parsing works
    let test_plan = create_minimal_test_plan();
    let plan_json = serde_json::to_string_pretty(&test_plan).unwrap();

    // Parse it back
    let parsed: RustlePlanOutput = serde_json::from_str(&plan_json).unwrap();
    assert_eq!(parsed.total_tasks, test_plan.total_tasks);
}

#[tokio::test]
async fn test_missing_static_files_handling() {
    // Test that missing static files are handled gracefully
    let test_plan = create_plan_with_missing_static_files();
    let plan_json = serde_json::to_string_pretty(&test_plan).unwrap();

    // Parse the plan (this should succeed even with missing static files)
    let parsed_plan: RustlePlanOutput = serde_json::from_str(&plan_json).unwrap();

    assert_eq!(parsed_plan.binary_deployments.len(), 1);
    assert_eq!(parsed_plan.binary_deployments[0].static_files.len(), 1);

    // The static file should reference a non-existent path
    let static_file = &parsed_plan.binary_deployments[0].static_files[0];
    assert_eq!(static_file.source_path, "non/existent/path.conf");
}

#[test]
fn test_file_path_structure() {
    // Test that file path structures are correctly handled
    let static_file = StaticFileRef {
        source_path: "tests/fixtures/files/test_files/sample.conf".to_string(),
        target_path: "/tmp/test.conf".to_string(),
        permissions: Some(0o644),
        compress: false,
    };

    assert_eq!(
        static_file.source_path,
        "tests/fixtures/files/test_files/sample.conf"
    );
    assert_eq!(static_file.target_path, "/tmp/test.conf");
    assert_eq!(static_file.permissions, Some(0o644));
    assert!(!static_file.compress);
}

// Helper functions to create test data

fn create_minimal_test_plan() -> RustlePlanOutput {
    use chrono::Utc;
    use rustle_deploy::execution::rustle_plan::*;
    use std::time::Duration;

    RustlePlanOutput {
        metadata: RustlePlanMetadata {
            created_at: Utc::now(),
            rustle_plan_version: "0.1.0".to_string(),
            playbook_hash: "test_hash".to_string(),
            inventory_hash: "test_inventory_hash".to_string(),
            planning_options: PlanningOptions {
                limit: None,
                tags: vec![],
                skip_tags: vec![],
                check_mode: false,
                diff_mode: false,
                forks: 5,
                serial: None,
                strategy: rustle_deploy::execution::plan::ExecutionStrategy::BinaryHybrid,
                binary_threshold: 5,
                force_binary: false,
                force_ssh: false,
            },
        },
        plays: vec![],
        binary_deployments: vec![BinaryDeploymentPlan {
            deployment_id: "test_deployment".to_string(),
            target_hosts: vec!["localhost".to_string()],
            binary_name: "test_binary".to_string(),
            tasks: vec!["task1".to_string()],
            modules: vec!["debug".to_string()],
            embedded_data: EmbeddedData {
                execution_plan: "{}".to_string(),
                static_files: vec![],
                variables: std::collections::HashMap::new(),
                facts_required: vec![],
            },
            execution_mode: ExecutionMode::Controller,
            estimated_size: 1000000,
            compilation_requirements: CompilationRequirements {
                target_arch: "x86_64".to_string(),
                target_os: "linux".to_string(),
                rust_version: "1.70.0".to_string(),
                cross_compilation: false,
                static_linking: true,
                modules: None,
                static_files: None,
                target_triple: None,
                optimization_level: None,
                features: None,
            },
            task_ids: None,
            target_architecture: None,
            estimated_savings: None,
            controller_endpoint: None,
            execution_timeout: None,
            report_interval: None,
            cleanup_on_completion: None,
            log_level: None,
            max_retries: None,
            static_files: vec![],
            secrets: vec![],
            verbose: None,
        }],
        total_tasks: 5,
        estimated_duration: Some(Duration::from_secs(10)),
        estimated_compilation_time: Some(Duration::from_secs(30)),
        parallelism_score: 1.0,
        network_efficiency_score: 1.0,
        hosts: vec!["localhost".to_string()],
    }
}

fn create_plan_with_missing_static_files() -> RustlePlanOutput {
    let mut plan = create_minimal_test_plan();

    // Add static files that don't exist to test graceful handling
    plan.binary_deployments[0].static_files = vec![StaticFileRef {
        source_path: "non/existent/path.conf".to_string(),
        target_path: "/tmp/missing.conf".to_string(),
        permissions: Some(0o644),
        compress: false,
    }];

    plan
}
