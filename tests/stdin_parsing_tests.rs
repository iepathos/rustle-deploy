use anyhow::Result;
use rustle_deploy::execution::rustle_plan::*;
use rustle_deploy::template::embedder::DataEmbedder;
use rustle_deploy::template::TemplateConfig;
use serde_json;
use std::collections::HashMap;
use tempfile::TempDir;
use tokio;

#[tokio::test]
async fn test_data_embedder_with_missing_static_files() {
    // Test that DataEmbedder handles missing static files gracefully
    let config = TemplateConfig::default();
    let embedder = DataEmbedder::new(&config).unwrap();

    let rustle_plan = create_test_rustle_plan();
    let mut binary_deployment = create_test_binary_deployment();

    // Add a static file reference that doesn't exist
    binary_deployment.static_files = vec![StaticFileRef {
        source_path: "tests/fixtures/files/test_files/nonexistent.conf".to_string(),
        target_path: "/tmp/test.conf".to_string(),
        permissions: Some(0o644),
        compress: false,
    }];

    let target_info = create_test_target_info();

    // This should succeed even with missing static files
    let result = embedder
        .embed_execution_data(&rustle_plan, &binary_deployment, &target_info)
        .await;

    match result {
        Ok(embedded_data) => {
            // Should have processed without the missing file
            assert!(embedded_data.static_files.is_empty());
            assert!(!embedded_data.execution_plan.is_empty());
        }
        Err(e) => {
            panic!(
                "embed_execution_data should not fail with missing static files: {}",
                e
            );
        }
    }
}

#[tokio::test]
async fn test_data_embedder_with_existing_static_files() {
    // Test that DataEmbedder correctly loads existing static files
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("test.conf");
    let test_content = "test configuration content";
    tokio::fs::write(&test_file_path, test_content)
        .await
        .unwrap();

    let config = TemplateConfig::default();
    let embedder = DataEmbedder::new(&config).unwrap();

    let rustle_plan = create_test_rustle_plan();
    let mut binary_deployment = create_test_binary_deployment();

    // Add a static file reference that exists
    binary_deployment.static_files = vec![StaticFileRef {
        source_path: test_file_path.to_string_lossy().to_string(),
        target_path: "/tmp/test.conf".to_string(),
        permissions: Some(0o644),
        compress: false,
    }];

    let target_info = create_test_target_info();

    let result = embedder
        .embed_execution_data(&rustle_plan, &binary_deployment, &target_info)
        .await;

    match result {
        Ok(embedded_data) => {
            // Should have loaded the static file
            assert_eq!(embedded_data.static_files.len(), 1);
            assert!(embedded_data.static_files.contains_key("/tmp/test.conf"));

            let loaded_content = &embedded_data.static_files["/tmp/test.conf"];
            assert_eq!(String::from_utf8_lossy(loaded_content), test_content);
        }
        Err(e) => {
            panic!(
                "embed_execution_data should succeed with existing static files: {}",
                e
            );
        }
    }
}

#[tokio::test]
async fn test_data_embedder_mixed_static_files() {
    // Test with a mix of existing and missing static files
    let temp_dir = TempDir::new().unwrap();
    let existing_file_path = temp_dir.path().join("existing.conf");
    let existing_content = "existing file content";
    tokio::fs::write(&existing_file_path, existing_content)
        .await
        .unwrap();

    let config = TemplateConfig::default();
    let embedder = DataEmbedder::new(&config).unwrap();

    let rustle_plan = create_test_rustle_plan();
    let mut binary_deployment = create_test_binary_deployment();

    // Add both existing and missing static file references
    binary_deployment.static_files = vec![
        StaticFileRef {
            source_path: existing_file_path.to_string_lossy().to_string(),
            target_path: "/tmp/existing.conf".to_string(),
            permissions: Some(0o644),
            compress: false,
        },
        StaticFileRef {
            source_path: "nonexistent/missing.conf".to_string(),
            target_path: "/tmp/missing.conf".to_string(),
            permissions: Some(0o644),
            compress: false,
        },
    ];

    let target_info = create_test_target_info();

    let result = embedder
        .embed_execution_data(&rustle_plan, &binary_deployment, &target_info)
        .await;

    match result {
        Ok(embedded_data) => {
            // Should have loaded only the existing file
            assert_eq!(embedded_data.static_files.len(), 1);
            assert!(embedded_data
                .static_files
                .contains_key("/tmp/existing.conf"));
            assert!(!embedded_data.static_files.contains_key("/tmp/missing.conf"));

            let loaded_content = &embedded_data.static_files["/tmp/existing.conf"];
            assert_eq!(String::from_utf8_lossy(loaded_content), existing_content);
        }
        Err(e) => {
            panic!(
                "embed_execution_data should succeed with mixed static files: {}",
                e
            );
        }
    }
}

#[test]
fn test_execution_plan_summary_creation() {
    // Test the create_execution_plan_summary logic
    let rustle_plan = create_test_rustle_plan_with_multiple_deployments();

    let total_tasks = rustle_plan.total_tasks;
    let binary_deployment_hosts: usize = rustle_plan
        .binary_deployments
        .iter()
        .map(|deployment| deployment.target_hosts.len())
        .sum();
    let all_hosts = rustle_plan.hosts.len();
    let ssh_fallback_hosts = all_hosts.saturating_sub(binary_deployment_hosts);

    // Calculate expected speedup
    let binary_ratio = if all_hosts > 0 {
        binary_deployment_hosts as f32 / all_hosts as f32
    } else {
        0.0
    };
    let expected_speedup = 1.0 + (binary_ratio * 4.0);

    assert_eq!(total_tasks, 10);
    assert_eq!(binary_deployment_hosts, 2); // 2 hosts in binary deployments
    assert_eq!(all_hosts, 3); // 3 total hosts
    assert_eq!(ssh_fallback_hosts, 1); // 1 fallback host
    assert!((expected_speedup - 3.667).abs() < 0.01); // ~3.67x speedup
}

#[test]
fn test_execution_plan_summary_no_binary_deployments() {
    // Test with no binary deployments (all SSH fallback)
    let mut rustle_plan = create_test_rustle_plan();
    rustle_plan.binary_deployments.clear();
    rustle_plan.hosts = vec!["host1".to_string(), "host2".to_string()];

    let binary_deployment_hosts: usize = rustle_plan
        .binary_deployments
        .iter()
        .map(|deployment| deployment.target_hosts.len())
        .sum();
    let all_hosts = rustle_plan.hosts.len();
    let ssh_fallback_hosts = all_hosts.saturating_sub(binary_deployment_hosts);

    assert_eq!(binary_deployment_hosts, 0);
    assert_eq!(all_hosts, 2);
    assert_eq!(ssh_fallback_hosts, 2);
}

#[test]
fn test_json_parsing_robustness() {
    // Test that we can parse various execution plan formats
    let minimal_json = r#"{
        "metadata": {
            "created_at": "2025-01-01T00:00:00Z",
            "rustle_plan_version": "0.1.0",
            "playbook_hash": "hash",
            "inventory_hash": "inv_hash",
            "planning_options": {
                "limit": null,
                "tags": [],
                "skip_tags": [],
                "check_mode": false,
                "diff_mode": false,
                "forks": 5,
                "serial": null,
                "strategy": "BinaryHybrid",
                "binary_threshold": 5,
                "force_binary": false,
                "force_ssh": false
            }
        },
        "plays": [],
        "binary_deployments": [],
        "total_tasks": 0,
        "estimated_duration": null,
        "estimated_compilation_time": null,
        "parallelism_score": 1.0,
        "network_efficiency_score": 1.0,
        "hosts": []
    }"#;

    let result: Result<RustlePlanOutput, _> = serde_json::from_str(minimal_json);
    assert!(result.is_ok(), "Should be able to parse minimal valid JSON");
}

// Helper functions to create test data

fn create_test_rustle_plan() -> RustlePlanOutput {
    use chrono::Utc;
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
        binary_deployments: vec![create_test_binary_deployment()],
        total_tasks: 5,
        estimated_duration: Some(Duration::from_secs(10)),
        estimated_compilation_time: Some(Duration::from_secs(30)),
        parallelism_score: 1.0,
        network_efficiency_score: 1.0,
        hosts: vec!["localhost".to_string()],
    }
}

fn create_test_rustle_plan_with_multiple_deployments() -> RustlePlanOutput {
    let mut plan = create_test_rustle_plan();
    plan.total_tasks = 10;
    plan.hosts = vec![
        "host1".to_string(),
        "host2".to_string(),
        "host3".to_string(),
    ];

    // Create two binary deployments with different hosts
    let mut deployment1 = create_test_binary_deployment();
    deployment1.deployment_id = "deployment1".to_string();
    deployment1.target_hosts = vec!["host1".to_string()];

    let mut deployment2 = create_test_binary_deployment();
    deployment2.deployment_id = "deployment2".to_string();
    deployment2.target_hosts = vec!["host2".to_string()];

    plan.binary_deployments = vec![deployment1, deployment2];
    plan
}

fn create_test_binary_deployment() -> BinaryDeploymentPlan {
    BinaryDeploymentPlan {
        deployment_id: "test_deployment".to_string(),
        target_hosts: vec!["localhost".to_string()],
        binary_name: "test_binary".to_string(),
        tasks: vec!["task1".to_string()],
        modules: vec!["debug".to_string()],
        embedded_data: EmbeddedData {
            execution_plan: r#"{"tasks": []}"#.to_string(),
            static_files: vec![],
            variables: HashMap::new(),
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
    }
}

fn create_test_target_info() -> rustle_deploy::template::TargetInfo {
    rustle_deploy::template::TargetInfo {
        target_triple: "x86_64-unknown-linux-gnu".to_string(),
        platform: rustle_deploy::types::platform::Platform::Linux,
        architecture: "x86_64".to_string(),
        os_family: "unix".to_string(),
        libc: Some("gnu".to_string()),
        features: vec![],
    }
}
