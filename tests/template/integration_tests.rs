use rustle_deploy::template::{BinaryTemplateGenerator, TemplateConfig, TargetInfo, OptimizationLevel};
use rustle_deploy::execution::rustle_plan::{RustlePlanOutput, BinaryDeploymentPlan};
use rustle_deploy::types::Platform;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::process::Command;

async fn create_test_template() -> (BinaryTemplateGenerator, RustlePlanOutput, BinaryDeploymentPlan, TargetInfo) {
    let config = TemplateConfig {
        optimization_level: OptimizationLevel::Debug,
        ..Default::default()
    };
    
    let generator = BinaryTemplateGenerator::new(config).expect("Failed to create generator");
    
    // Create a minimal test execution plan
    let execution_plan = serde_json::from_str(r#"{
        "metadata": {
            "created_at": "2024-01-01T00:00:00Z",
            "rustle_plan_version": "1.0.0",
            "playbook_hash": "test-hash",
            "inventory_hash": "test-inventory-hash",
            "planning_options": {
                "limit": null,
                "tags": [],
                "skip_tags": [],
                "check_mode": false,
                "diff_mode": false,
                "forks": 5,
                "serial": null,
                "strategy": "Linear",
                "binary_threshold": 10,
                "force_binary": false,
                "force_ssh": false
            }
        },
        "plays": [{
            "play_id": "play-1",
            "name": "Test Play",
            "strategy": "Linear",
            "serial": null,
            "hosts": ["localhost"],
            "batches": [{
                "batch_id": "batch-1",
                "hosts": ["localhost"],
                "tasks": [{
                    "task_id": "task-1",
                    "name": "Debug task",
                    "module": "debug",
                    "args": {
                        "msg": "Hello World"
                    },
                    "hosts": ["localhost"],
                    "dependencies": [],
                    "conditions": [],
                    "tags": [],
                    "notify": [],
                    "execution_order": 1,
                    "can_run_parallel": true,
                    "estimated_duration": {"secs": 5, "nanos": 0},
                    "risk_level": "Low"
                }],
                "parallel_groups": [],
                "dependencies": [],
                "estimated_duration": {"secs": 10, "nanos": 0}
            }],
            "handlers": [],
            "estimated_duration": {"secs": 20, "nanos": 0}
        }],
        "binary_deployments": [],
        "total_tasks": 1,
        "estimated_duration": {"secs": 30, "nanos": 0},
        "estimated_compilation_time": {"secs": 60, "nanos": 0},
        "parallelism_score": 0.8,
        "network_efficiency_score": 0.9,
        "hosts": ["localhost"]
    }"#).expect("Failed to parse test execution plan");
    
    let binary_deployment = BinaryDeploymentPlan {
        deployment_id: "test-deployment".to_string(),
        target_hosts: vec!["localhost".to_string()],
        target_architecture: "x86_64".to_string(),
        task_ids: vec!["task-1".to_string()],
        estimated_savings: std::time::Duration::from_secs(30),
        compilation_requirements: rustle_deploy::execution::rustle_plan::CompilationRequirements {
            modules: vec!["debug".to_string()],
            static_files: vec![],
            target_triple: "x86_64-unknown-linux-gnu".to_string(),
            optimization_level: "debug".to_string(),
            features: vec![],
        },
        controller_endpoint: None,
        execution_timeout: Some(std::time::Duration::from_secs(300)),
        report_interval: Some(std::time::Duration::from_secs(30)),
        cleanup_on_completion: Some(false), // Don't cleanup for testing
        log_level: Some("debug".to_string()),
        max_retries: Some(3),
        static_files: vec![],
        secrets: vec![],
    };
    
    let target_info = TargetInfo {
        target_triple: "x86_64-unknown-linux-gnu".to_string(),
        platform: Platform::Linux,
        architecture: "x86_64".to_string(),
        os_family: "unix".to_string(),
        libc: Some("glibc".to_string()),
        features: vec![],
    };
    
    (generator, execution_plan, binary_deployment, target_info)
}

#[tokio::test]
async fn test_end_to_end_template_generation() {
    let (generator, execution_plan, binary_deployment, target_info) = create_test_template().await;
    
    let template = generator
        .generate_binary_template(&execution_plan, &binary_deployment, &target_info)
        .await
        .expect("Failed to generate template");
    
    // Verify all expected files are present
    let expected_files = [
        "src/main.rs",
        "src/modules/debug.rs",
    ];
    
    for expected_file in &expected_files {
        let path = PathBuf::from(expected_file);
        assert!(
            template.source_files.contains_key(&path),
            "Expected file {} not found in template",
            expected_file
        );
    }
    
    // Verify template can be written to filesystem
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_dir = temp_dir.path().join("test-project");
    fs::create_dir_all(&project_dir).expect("Failed to create project directory");
    
    // Write all source files
    for (file_path, content) in &template.source_files {
        let full_path = project_dir.join(file_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directory");
        }
        fs::write(full_path, content).expect("Failed to write source file");
    }
    
    // Write Cargo.toml
    fs::write(project_dir.join("Cargo.toml"), &template.cargo_toml)
        .expect("Failed to write Cargo.toml");
    
    // Verify the project structure is valid
    assert!(project_dir.join("src/main.rs").exists());
    assert!(project_dir.join("Cargo.toml").exists());
}

#[tokio::test]
#[ignore] // This test requires cargo to be installed and takes time
async fn test_generated_template_compiles() {
    let (generator, execution_plan, binary_deployment, target_info) = create_test_template().await;
    
    let template = generator
        .generate_binary_template(&execution_plan, &binary_deployment, &target_info)
        .await
        .expect("Failed to generate template");
    
    // Create temporary project directory
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_dir = temp_dir.path().join("test-project");
    fs::create_dir_all(&project_dir).expect("Failed to create project directory");
    
    // Write all files
    for (file_path, content) in &template.source_files {
        let full_path = project_dir.join(file_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directory");
        }
        fs::write(full_path, content).expect("Failed to write source file");
    }
    
    fs::write(project_dir.join("Cargo.toml"), &template.cargo_toml)
        .expect("Failed to write Cargo.toml");
    
    // Try to compile the project
    let output = Command::new("cargo")
        .args(&["check", "--quiet"])
        .current_dir(&project_dir)
        .output()
        .await
        .expect("Failed to run cargo check");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Generated template failed to compile:\nSTDOUT:\n{}\nSTDERR:\n{}",
            stdout, stderr
        );
    }
}

#[tokio::test]
async fn test_template_with_multiple_modules() {
    let (generator, mut execution_plan, binary_deployment, target_info) = create_test_template().await;
    
    // Add more tasks with different modules
    execution_plan.plays[0].batches[0].tasks.extend(vec![
        rustle_deploy::execution::rustle_plan::TaskPlan {
            task_id: "task-2".to_string(),
            name: "Command task".to_string(),
            module: "command".to_string(),
            args: {
                let mut args = std::collections::HashMap::new();
                args.insert("cmd".to_string(), serde_json::Value::String("echo test".to_string()));
                args
            },
            hosts: vec!["localhost".to_string()],
            dependencies: vec![],
            conditions: vec![],
            tags: vec![],
            notify: vec![],
            execution_order: 2,
            can_run_parallel: true,
            estimated_duration: std::time::Duration::from_secs(5),
            risk_level: rustle_deploy::execution::rustle_plan::RiskLevel::Low,
        },
        rustle_deploy::execution::rustle_plan::TaskPlan {
            task_id: "task-3".to_string(),
            name: "Package task".to_string(),
            module: "package".to_string(),
            args: {
                let mut args = std::collections::HashMap::new();
                args.insert("name".to_string(), serde_json::Value::String("curl".to_string()));
                args.insert("state".to_string(), serde_json::Value::String("present".to_string()));
                args
            },
            hosts: vec!["localhost".to_string()],
            dependencies: vec![],
            conditions: vec![],
            tags: vec![],
            notify: vec![],
            execution_order: 3,
            can_run_parallel: false,
            estimated_duration: std::time::Duration::from_secs(10),
            risk_level: rustle_deploy::execution::rustle_plan::RiskLevel::Medium,
        },
    ]);
    
    execution_plan.total_tasks = 3;
    
    let template = generator
        .generate_binary_template(&execution_plan, &binary_deployment, &target_info)
        .await
        .expect("Failed to generate template");
    
    // Verify all module implementations are present
    let expected_modules = ["debug", "command", "package"];
    for module in &expected_modules {
        let module_path = PathBuf::from(format!("src/modules/{}.rs", module));
        assert!(
            template.source_files.contains_key(&module_path),
            "Module {} not found in template",
            module
        );
    }
    
    // Verify main.rs contains module declarations
    let main_rs = template.source_files.get(&PathBuf::from("src/main.rs")).unwrap();
    for module in &expected_modules {
        assert!(
            main_rs.contains(&format!("modules::{}::execute", module)),
            "Module {} not referenced in main.rs",
            module
        );
    }
}

#[tokio::test]
async fn test_template_size_estimation() {
    let (generator, execution_plan, binary_deployment, target_info) = create_test_template().await;
    
    let template = generator
        .generate_binary_template(&execution_plan, &binary_deployment, &target_info)
        .await
        .expect("Failed to generate template");
    
    // Verify size estimation is reasonable
    assert!(template.estimated_binary_size > 1_000_000); // At least 1MB
    assert!(template.estimated_binary_size < 100_000_000); // Less than 100MB
}

#[tokio::test]
async fn test_platform_specific_generation() {
    let config = TemplateConfig::default();
    let generator = BinaryTemplateGenerator::new(config).expect("Failed to create generator");
    
    let execution_plan = serde_json::from_str(r#"{
        "metadata": {
            "created_at": "2024-01-01T00:00:00Z",
            "rustle_plan_version": "1.0.0",
            "playbook_hash": "test-hash",
            "inventory_hash": "test-inventory-hash",
            "planning_options": {
                "limit": null,
                "tags": [],
                "skip_tags": [],
                "check_mode": false,
                "diff_mode": false,
                "forks": 5,
                "serial": null,
                "strategy": "Linear",
                "binary_threshold": 10,
                "force_binary": false,
                "force_ssh": false
            }
        },
        "plays": [],
        "binary_deployments": [],
        "total_tasks": 0,
        "estimated_duration": null,
        "estimated_compilation_time": null,
        "parallelism_score": 0.0,
        "network_efficiency_score": 0.0,
        "hosts": []
    }"#).expect("Failed to parse test execution plan");
    
    let binary_deployment = BinaryDeploymentPlan {
        deployment_id: "test-deployment".to_string(),
        target_hosts: vec![],
        target_architecture: "x86_64".to_string(),
        task_ids: vec![],
        estimated_savings: std::time::Duration::from_secs(0),
        compilation_requirements: rustle_deploy::execution::rustle_plan::CompilationRequirements {
            modules: vec![],
            static_files: vec![],
            target_triple: "x86_64-unknown-linux-gnu".to_string(),
            optimization_level: "release".to_string(),
            features: vec![],
        },
        controller_endpoint: None,
        execution_timeout: None,
        report_interval: None,
        cleanup_on_completion: None,
        log_level: None,
        max_retries: None,
        static_files: vec![],
        secrets: vec![],
    };
    
    // Test different platforms
    let platforms = vec![
        (Platform::Linux, "x86_64-unknown-linux-gnu"),
        (Platform::Windows, "x86_64-pc-windows-msvc"),
        (Platform::MacOS, "x86_64-apple-darwin"),
    ];
    
    for (platform, target_triple) in platforms {
        let target_info = TargetInfo {
            target_triple: target_triple.to_string(),
            platform: platform.clone(),
            architecture: "x86_64".to_string(),
            os_family: match platform {
                Platform::Windows => "windows",
                Platform::MacOS => "unix",
                Platform::Linux => "unix",
                _ => "unknown",
            }.to_string(),
            libc: match platform {
                Platform::Linux => Some("glibc".to_string()),
                _ => None,
            },
            features: vec![],
        };
        
        let template = generator
            .generate_binary_template(&execution_plan, &binary_deployment, &target_info)
            .await
            .expect(&format!("Failed to generate template for {:?}", platform));
        
        // Verify platform-specific content
        assert_eq!(template.target_info.platform, platform);
        assert_eq!(template.target_info.target_triple, target_triple);
        
        // Verify Cargo.toml contains correct target
        if target_triple != "x86_64-unknown-linux-gnu" {
            assert!(template.cargo_toml.contains(target_triple));
        }
    }
}